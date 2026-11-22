//! DevTools 开发者工具箱入口。
//!
//! 启动流程：
//! 1. 加载共享配置（沿用历史版本 JSON 文件，双向兼容）
//! 2. 初始化 gpui-component、主题、tokio 桥
//! 3. 设置应用菜单栏（关于/设置/编辑/窗口/帮助）
//! 4. 按配置创建托盘
//! 5. 打开主窗口（--autostart / start_minimized 时隐藏）

mod about;
mod app_menu;
mod autostart;
mod components;
mod config_state;
mod core_rt;
mod macos_window_ctl;
mod main_window;
mod pages;
mod theme;
mod tool_window;
mod tray;
mod windows;

use devtools::config::AppConfig;
use gpui::App;

fn main() {
    let config = AppConfig::load();
    let autostarted = std::env::args().any(|arg| arg == "--autostart");
    let start_minimized = config.start_minimized || autostarted;

    let app = gpui_platform::application().with_assets(gpui_kit_assets::AllAssets);
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);

        config_state::init(config.clone(), cx);
        theme::apply_initial(config.theme, cx);
        core_rt::init(cx);
        app_menu::init(cx);

        windows::init(cx);
        windows::create_main_window(cx, !start_minimized);
        // 非 minimized 启动时激活应用（启动即前置主窗口）
        if !start_minimized {
            cx.activate(true);
        }

        if config.tray_enabled {
            tray::init(cx);
        }
    });
}
