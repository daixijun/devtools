//! DevTools 开发者工具箱入口。
//!
//! 启动流程：
//! 1. 单例抢占：已有实例则激活其主窗口后退出
//! 2. 加载共享配置（沿用历史版本 JSON 文件，双向兼容）
//! 3. 初始化 gpui-component、主题、tokio 桥
//! 4. 设置应用菜单栏（关于/设置/编辑/窗口/帮助）
//! 5. 按配置创建托盘
//! 6. 打开主窗口（--autostart / start_minimized 时隐藏）

mod about;
mod app_menu;
mod autostart;
mod components;
mod config_state;
mod core_rt;
mod main_window;
mod pages;
mod singleton;
mod theme;
mod tool_window;
mod tray;
mod update;
mod window_ctl;
mod windows;

use devtools::config::AppConfig;
use gpui::App;

fn main() {
    // Windows 自更新残留（旧版本 exe 改名让位后的备份）：启动时尝试清理
    #[cfg(target_os = "windows")]
    if let Ok(exe) = std::env::current_exe() {
        let old = exe.with_extension("exe.old");
        if old.exists() {
            let _ = std::fs::remove_file(old);
        }
    }

    // 单例：二次启动时激活已有实例的主窗口并退出
    if !singleton::acquire_or_notify() {
        return;
    }

    let config = AppConfig::load();
    devtools::i18n::init(&config.language);
    let autostarted = std::env::args().any(|arg| arg == "--autostart");
    let start_minimized = config.start_minimized || autostarted;

    let app = gpui_platform::application().with_assets(gpui_kit_assets::AllAssets);
    // Dock 图标点击（applicationShouldHandleReopen）：主窗口被隐藏时
    // （关闭入托盘 / Cmd-H / 最小化启动）唤回并前置——与单例激活同一路径。
    // 仅 macOS 有 Dock 语义；回调注册在 Application 上（run 之前），
    // 执行于 run 后的 App 上下文
    #[cfg(target_os = "macos")]
    app.on_reopen(windows::show_main);
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);

        config_state::init(config.clone(), cx);
        theme::apply_initial(config.theme, cx);
        core_rt::init(cx);
        app_menu::init(cx);

        windows::init(cx);
        windows::create_main_window(cx, !start_minimized);
        singleton::init(cx);
        // 自动更新（设置页开启时）：静默检查下载，完成后弹窗提示确认重启
        update::auto_update_on_launch(cx);
        // 非 minimized 启动时激活应用（启动即前置主窗口）
        if !start_minimized {
            cx.activate(true);
        }

        if config.tray_enabled {
            tray::init(cx);
        }
    });
}
