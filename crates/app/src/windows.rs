//! 窗口管理（全局单例）。
//!
//! 主窗口形态：单主窗口（左侧分类导航 + 右侧工具内容区，见 main_window.rs）。
//! `open_tool` 在主窗口右侧内容区切换工具（不再每工具开新窗）；
//! 另有「关于」小窗口（app 菜单打开）。

use crate::{macos_window_ctl, about::AboutView, main_window::MainWindow};
use devtools_core::registry;
use gpui::{
    px, size, App, AppContext as _, Bounds, Entity, Global, WindowBounds, WindowHandle,
    WindowOptions,
};
use gpui_component::Root;
use std::rc::Rc;

#[derive(Default)]
pub struct WindowManager {
    pub(crate) main: Option<WindowHandle<Root>>,
    /// 主窗口视图强引用：主窗口与 app 同生命周期（仅隐藏，不关闭）
    main_view: Option<Entity<MainWindow>>,
    about: Option<WindowHandle<Root>>,
}

impl Global for WindowManager {}

pub fn init(cx: &mut App) {
    cx.set_global(WindowManager::default());
}

/// 主显示器居中的指定尺寸窗口边界。
fn centered_bounds(cx: &App, width: f32, height: f32) -> Bounds<gpui::Pixels> {
    let win_size = size(px(width), px(height));
    let display_bounds = cx
        .primary_display()
        .map(|display| display.bounds())
        .unwrap_or_default();
    Bounds::new(
        gpui::Point::new(
            display_bounds.origin.x + (display_bounds.size.width - win_size.width) / 2.,
            display_bounds.origin.y + (display_bounds.size.height - win_size.height) / 2.,
        ),
        win_size,
    )
}

/// 创建主窗口。
pub fn create_main_window(cx: &mut App, visible: bool) {
    let view_slot: Rc<std::cell::RefCell<Option<Entity<MainWindow>>>> = Default::default();
    let slot_for_closure = view_slot.clone();
    let bounds = centered_bounds(cx, 1080., 720.);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("开发者工具箱 - DevTools".into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        focus: true,
        show: visible,
        kind: gpui::WindowKind::Normal,
        is_movable: true,
        app_owns_titlebar_drag: false,
        inactive_frame_interval: None,
        low_power_frame_interval: None,
        thermal_frame_interval: None,
        is_resizable: true,
        is_minimizable: true,
        display_id: None,
        window_background: gpui::WindowBackgroundAppearance::Opaque,
        app_id: None,
        window_min_size: Some(size(px(880.), px(560.))),
        window_decorations: None,
        icon: None,
        tabbing_identifier: None,
        use_display_p3: false,
    };

    let opened = cx.open_window(options, |window, cx| {
        crate::theme::observe_window_appearance(window, cx);
        let view = cx.new(|cx| MainWindow::new(window, cx));
        *slot_for_closure.borrow_mut() = Some(view.clone());
        cx.new(|cx| Root::new(gpui::AnyView::from(view), window, cx))
    });

    match opened {
        Ok(handle) => {
            // close-to-tray：拦截主窗口关闭请求
            handle
                .update(cx, |_, window, cx| {
                    window.on_window_should_close(cx, |window, cx| {
                        let config = crate::config::get(cx);
                        if config.tray_enabled && config.close_to_tray {
                            macos_window_ctl::hide(window);
                            false
                        } else {
                            cx.quit();
                            true
                        }
                    });
                })
                .ok();
            let wm = cx.global_mut::<WindowManager>();
            wm.main = Some(handle);
            wm.main_view = view_slot.borrow().clone();
        }
        Err(e) => eprintln!("创建主窗口失败: {}", e),
    }
}

/// 显示并聚焦主窗口。
pub fn show_main(cx: &mut App) {
    let Some(handle) = cx.global::<WindowManager>().main else {
        return;
    };
    let view = cx.global::<WindowManager>().main_view.clone();
    let _ = handle.update(cx, |_, window, cx| {
        macos_window_ctl::show(window);
        window.activate_window();
        cx.activate(true);
        // 聚焦搜索框（handle.update 闭包持有 window；focus 无需 view 的可变上下文）
        if let Some(view) = view {
            let focus_handle = view.read(cx).input_focus_handle().clone();
            focus_handle.focus(window, cx);
            cx.notify();
        }
    });
}

/// 隐藏主窗口（app 菜单「隐藏」动作）。
pub fn hide_main(cx: &mut App) {
    let Some(handle) = cx.global::<WindowManager>().main else {
        return;
    };
    let _ = handle.update(cx, |_, window, _| macos_window_ctl::hide(window));
}

/// 在主窗口右侧内容区打开（切换到）指定工具；主窗口不可见时先显示。
///
/// 先做纯 entity 操作记录待切换工具（click 回调等窗口 dispatch 中调用的
/// 路径里 `handle.update` 会因窗口正被借用而失败，故与之解耦）；
/// 再尝试显示/激活窗口（托盘/菜单路径需要，dispatch 中会静默失败但窗口本就可见）。
pub fn open_tool(cx: &mut App, tool_id: &str) {
    let Some(tool) = registry::get_tool_by_id(tool_id) else {
        eprintln!("未知工具: {}", tool_id);
        return;
    };

    if let Some(view) = cx.global::<WindowManager>().main_view.clone() {
        view.update(cx, |view, cx| view.select_tool(tool, cx));
    }

    if let Some(handle) = cx.global::<WindowManager>().main {
        let _ = handle.update(cx, |_, window, cx| {
            macos_window_ctl::show(window);
            window.activate_window();
            cx.activate(true);
        });
    }
}

/// 打开（或聚焦已存在的）「关于」窗口。
pub fn open_about(cx: &mut App) {
    if let Some(handle) = cx.global::<WindowManager>().about {
        let ok = handle.update(cx, |_, window, cx| {
            macos_window_ctl::show(window);
            window.activate_window();
            cx.activate(true);
        });
        if ok.is_ok() {
            return;
        }
    }

    let bounds = centered_bounds(cx, 380., 240.);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("关于".into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        focus: true,
        show: true,
        kind: gpui::WindowKind::Normal,
        is_movable: true,
        app_owns_titlebar_drag: false,
        inactive_frame_interval: None,
        low_power_frame_interval: None,
        thermal_frame_interval: None,
        is_resizable: false,
        is_minimizable: false,
        display_id: None,
        window_background: gpui::WindowBackgroundAppearance::Opaque,
        app_id: None,
        window_min_size: None,
        window_decorations: None,
        icon: None,
        tabbing_identifier: None,
        use_display_p3: false,
    };

    let opened = cx.open_window(options, |window, cx| {
        crate::theme::observe_window_appearance(window, cx);
        let view = cx.new(|cx| AboutView::new(window, cx));
        cx.new(|cx| Root::new(gpui::AnyView::from(view), window, cx))
    });

    match opened {
        Ok(handle) => {
            cx.global_mut::<WindowManager>().about = Some(handle);
        }
        Err(e) => eprintln!("打开关于窗口失败: {}", e),
    }
}
