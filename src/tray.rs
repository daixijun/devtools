//! 系统托盘（tray-icon + muda）。
//!
//! 菜单结构：7 分类二级菜单（30 工具）+ 设置 + 显示/退出。
//! 事件泵：muda MenuEvent 与 tray_icon TrayIconEvent 的全局 receiver 由后台线程
//! 转发进 gpui executor（AsyncApp）。托盘句柄故意泄漏以保持存活（泄漏即生命周期）。

use gpui::{App, Global};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

/// 托盘图标：32x32 PNG（include_bytes 编译期内嵌）。
const TRAY_ICON_PNG: &[u8] = include_bytes!("../assets/tray-32x32.png");

/// 托盘句柄全局单例（Settings 页可实时开关托盘）。
/// tray-icon 0.21 无 is_visible getter，可见性自行跟踪。
struct TrayState {
    tray: Option<tray_icon::TrayIcon>,
    visible: bool,
}

impl Global for TrayState {}

pub fn init(cx: &mut App) {
    let menu = build_menu();
    let icon = match load_icon() {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("加载托盘图标失败: {}（跳过托盘初始化）", e);
            cx.set_global(TrayState {
                tray: None,
                visible: false,
            });
            return;
        }
    };

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("DevTools 开发者工具箱")
        // 左键单击唤起主窗口（发 Click 事件），右键弹出菜单
        .with_menu_on_left_click(false)
        .build();

    match tray {
        Ok(tray) => {
            cx.set_global(TrayState {
                tray: Some(tray),
                visible: true,
            });
        }
        Err(e) => {
            eprintln!("创建托盘失败: {}（跳过托盘初始化）", e);
            cx.set_global(TrayState {
                tray: None,
                visible: false,
            });
            return;
        }
    }

    // 菜单事件泵：后台线程转发事件，gpui 前台任务消费（AsyncApp 非 Send，
    // 不能直接被 std::thread 持有，与 hotkey.rs 同构）
    let (menu_tx, mut menu_rx) = futures::channel::mpsc::unbounded::<MenuEvent>();
    std::thread::spawn(move || {
        while let Ok(event) = MenuEvent::receiver().recv() {
            if menu_tx.unbounded_send(event).is_err() {
                break;
            }
        }
    });
    cx.spawn(async move |cx| {
        while let Some(event) = futures::StreamExt::next(&mut menu_rx).await {
            let id = event.id().0.clone();
            cx.update(|cx| handle_menu_id(&id, cx));
        }
    })
    .detach();

    // 托盘左键点击泵：唤起主窗口
    let (click_tx, mut click_rx) = futures::channel::mpsc::unbounded::<TrayIconEvent>();
    std::thread::spawn(move || {
        while let Ok(event) = TrayIconEvent::receiver().recv() {
            if click_tx.unbounded_send(event).is_err() {
                break;
            }
        }
    });
    cx.spawn(async move |cx| {
        while let Some(event) = futures::StreamExt::next(&mut click_rx).await {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                cx.update(crate::windows::show_main);
            }
        }
    })
    .detach();
}

/// 实时开关托盘图标（立即生效，返回实际状态）。
pub fn set_enabled(enabled: bool, cx: &mut App) -> bool {
    let Some(state) = cx.try_global::<TrayState>() else {
        return false;
    };
    let Some(tray) = state.tray.as_ref() else {
        return false;
    };
    match tray.set_visible(enabled) {
        Ok(_) => {
            cx.global_mut::<TrayState>().visible = enabled;
            enabled
        }
        Err(e) => {
            eprintln!("切换托盘可见性失败: {e}");
            !enabled
        }
    }
}

/// 当前托盘是否可见。
pub fn is_enabled(cx: &App) -> bool {
    cx.try_global::<TrayState>()
        .map(|s| s.visible)
        .unwrap_or(false)
}

/// 构建 7 分类二级菜单（30 工具）+ 设置 + 显示/退出。
///
/// 「设置」不套子菜单：settings 分类只有一个入口，直接作为菜单项挂在
/// 分类之后（id 复用 `tool:` 前缀，经 open_tool("settings") 打开设置弹窗）。
fn build_menu() -> Menu {
    let menu = Menu::new();

    for category in devtools::registry::CATEGORIES {
        if category.id == "settings" {
            continue;
        }
        let mut items: Vec<MenuItem> = Vec::new();
        for tool in devtools::registry::tools_in_category(category.id) {
            items.push(MenuItem::with_id(
                format!("tool:{}", tool.id),
                tool.name,
                true,
                None,
            ));
        }
        if items.is_empty() {
            continue;
        }
        let item_refs: Vec<&dyn tray_icon::menu::IsMenuItem> = items
            .iter()
            .map(|i| i as &dyn tray_icon::menu::IsMenuItem)
            .collect();
        let Ok(submenu) = Submenu::with_items(category.name, true, &item_refs) else {
            continue;
        };
        let _ = menu.append(&submenu);
    }

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("tool:settings", "设置", true, None));
    let _ = menu.append(&MenuItem::with_id("show", "显示", true, None));
    let _ = menu.append(&MenuItem::with_id("quit", "退出", true, None));
    menu
}

fn handle_menu_id(id: &str, cx: &mut App) {
    match id {
        "show" => crate::windows::show_main(cx),
        "quit" => cx.quit(),
        tool_id if tool_id.starts_with("tool:") => {
            crate::windows::open_tool(cx, &tool_id[5..]);
        }
        _ => {}
    }
}

fn load_icon() -> Result<tray_icon::Icon, String> {
    let img = image::load_from_memory(TRAY_ICON_PNG)
        .map_err(|e| format!("解码托盘图标: {e}"))?
        .to_rgba8();
    let (width, height) = (img.width(), img.height());
    tray_icon::Icon::from_rgba(img.into_raw(), width, height).map_err(|e| e.to_string())
}
