//! EN 词条：壳层（应用菜单/托盘/窗口标题/主窗口/设置页/更新/关于）。
//!
//! 中文原文即 key；en 译法在语序需要处带空格（如「个工具」→ "tools"，
//! 调用点 `format!("{} {}", n, t("个工具"))`）。

/// 壳层词条表。
pub const EN_SHELL: &[(&str, &str)] = &[
    // ---- 应用菜单（app_menu.rs）----
    ("DevTools", "DevTools"),
    ("关于 DevTools", "About DevTools"),
    ("检查更新…", "Check for Updates…"),
    ("设置…", "Settings…"),
    ("隐藏 DevTools", "Hide DevTools"),
    ("退出 DevTools", "Quit DevTools"),
    ("编辑", "Edit"),
    ("撤销", "Undo"),
    ("重做", "Redo"),
    ("剪切", "Cut"),
    ("拷贝", "Copy"),
    ("粘贴", "Paste"),
    ("全选", "Select All"),
    ("窗口", "Window"),
    ("最小化", "Minimize"),
    ("关闭窗口", "Close Window"),
    ("帮助", "Help"),
    ("DevTools 帮助", "DevTools Help"),
    // ---- 托盘（tray.rs）----
    ("设置", "Settings"),
    ("显示", "Show"),
    ("退出", "Quit"),
    // ---- 窗口标题（windows.rs）----
    ("DevTools 开发者工具箱", "DevTools"),
    ("关于", "About"),
    ("软件更新", "Software Update"),
    // ---- 主窗口（main_window.rs）----
    ("搜索工具…", "Search tools…"),
    ("没有匹配的工具", "No matching tools"),
    ("个工具", "tools"),
    ("个分类", "categories"),
    ("纯 Rust 原生渲染", "pure Rust native rendering"),
    ("常用工具", "Favorites"),
    ("偏好设置", "Preferences"),
    ("主题 · 托盘 · 自启", "Theme · Tray · Autostart"),
    (
        "输入即搜索 · ↑↓ 选择 · Enter 打开 · 关闭窗口可隐藏到托盘",
        "Type to search · ↑↓ select · Enter to open · closing hides to tray",
    ),
    // ---- 设置页（settings.rs）----
    ("外观设置", "Appearance"),
    ("主题模式", "Theme Mode"),
    ("☀️ 浅色", "☀️ Light"),
    ("🌙 深色", "🌙 Dark"),
    ("🖥️ 跟随系统", "🖥️ System"),
    ("当前主题", "Current theme"),
    ("深色", "Dark"),
    ("浅色", "Light"),
    ("系统设置", "System"),
    ("显示系统托盘图标", "Show tray icon"),
    (
        "在系统托盘中显示应用图标，便于快速访问",
        "Show the app icon in the system tray for quick access",
    ),
    ("开机自动启动", "Launch at login"),
    (
        "系统启动时自动运行此应用程序",
        "Start the app automatically at system startup",
    ),
    ("启动时最小化到托盘", "Minimize to tray on launch"),
    (
        "应用程序启动时直接最小化到系统托盘",
        "Minimize to the tray when the app starts",
    ),
    ("关闭时最小化到托盘", "Minimize to tray on close"),
    (
        "关闭窗口时最小化到系统托盘而非退出程序",
        "Minimize to the tray instead of quitting when closed",
    ),
    ("关于应用", "About the app"),
    ("应用名称", "App name"),
    ("开发者工具箱", "DevTools"),
    ("版本", "Version"),
    ("技术栈", "Tech stack"),
    ("语言", "Language"),
    ("跟随系统", "System"),
    ("简体中文", "简体中文"),
    ("English", "English"),
    (
        "托盘状态切换失败，已回滚",
        "Failed to switch tray state, rolled back",
    ),
    ("设置开机自启失败", "Failed to set launch at login"),
    // ---- 组件层（ui.rs）----
    ("复制", "Copy"),
    ("已复制", "Copied"),
    // ---- 软件更新窗口（update.rs）----
    ("正在检查更新…", "Checking for updates…"),
    ("已是最新版本", "You're up to date"),
    ("当前版本", "Current version"),
    ("发现新版本", "New version available"),
    ("前往下载", "Download"),
    ("检查更新失败", "Update check failed"),
    ("重试", "Retry"),
    // ---- 关于窗口（about.rs）----
    (
        "面向开发者的效率工具箱，内置 31 个常用工具",
        "A developer toolbox with 31 handy tools",
    ),
    // ---- 更新检查业务层（updater.rs 404 提示）----
    (
        "GitHub Releases（仓库还没有已发布的正式版本）",
        "GitHub Releases (no published release yet)",
    ),
];
