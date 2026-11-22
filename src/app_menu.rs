//! macOS 应用菜单栏（关于/设置/编辑/窗口/帮助）。
//!
//! 菜单项 action 通过 `cx.dispatch_action` 全局分发：注册 `cx.on_action`
//! 全局监听实现菜单行为；「编辑」菜单使用 gpui-component 输入组件自带的
//! 剪贴板 action + OsAction（系统 selector），快捷键展示来自输入组件默认 keymap。

use gpui::{actions, App, KeyBinding, Menu, MenuItem, OsAction};
use gpui_component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};

use crate::windows;

actions!(
    devtools_app,
    [
        About,
        OpenSettings,
        HideApp,
        QuitApp,
        MinimizeWindow,
        CloseWindow,
        OpenHelp
    ]
);

pub fn init(cx: &mut App) {
    // 菜单快捷键（macOS 规范绑定；菜单项的快捷键展示与系统触发都来自 keymap）
    cx.bind_keys([
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", QuitApp, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
    ]);

    cx.set_menus([
        Menu::new("DevTools").items([
            MenuItem::action("关于 DevTools", About),
            MenuItem::separator(),
            MenuItem::action("设置…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("隐藏 DevTools", HideApp),
            MenuItem::separator(),
            MenuItem::action("退出 DevTools", QuitApp),
        ]),
        Menu::new("编辑").items([
            MenuItem::os_action("撤销", Undo, OsAction::Undo),
            MenuItem::os_action("重做", Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("剪切", Cut, OsAction::Cut),
            MenuItem::os_action("拷贝", Copy, OsAction::Copy),
            MenuItem::os_action("粘贴", Paste, OsAction::Paste),
            MenuItem::os_action("全选", SelectAll, OsAction::SelectAll),
        ]),
        Menu::new("窗口").items([
            MenuItem::action("最小化", MinimizeWindow),
            MenuItem::action("关闭窗口", CloseWindow),
        ]),
        Menu::new("帮助").items([MenuItem::action("DevTools 帮助", OpenHelp)]),
    ]);

    cx.on_action(|_: &About, cx| windows::open_about(cx));
    cx.on_action(|_: &OpenSettings, cx| windows::open_tool(cx, "settings"));
    cx.on_action(|_: &HideApp, cx| windows::hide_main(cx));
    cx.on_action(|_: &QuitApp, cx| cx.quit());
    cx.on_action(|_: &MinimizeWindow, cx| minimize_active_window(cx));
    cx.on_action(|_: &CloseWindow, cx| close_active_window(cx));
    cx.on_action(|_: &OpenHelp, cx| cx.open_url("https://devtools.linuxyunwei.com"));
}

/// 最小化当前活动窗口。
fn minimize_active_window(cx: &mut App) {
    if let Some(window) = cx.active_window() {
        let _ = window.update(cx, |_, window, _| window.minimize_window());
    }
}

/// 关闭当前活动窗口：主窗口按 close-to-tray 配置隐藏到托盘，其余窗口直接关闭。
fn close_active_window(cx: &mut App) {
    let wm = cx.global::<crate::windows::WindowManager>();
    let main_id = wm.main.as_ref().map(|handle| handle.window_id());
    if let Some(window) = cx.active_window() {
        let is_main = main_id == Some(window.window_id());
        let _ = window.update(cx, |_, window, cx| {
            if is_main {
                let config = crate::config_state::get(cx);
                if config.tray_enabled && config.close_to_tray {
                    crate::macos_window_ctl::hide(window);
                    return;
                }
            }
            window.remove_window();
        });
    }
}
