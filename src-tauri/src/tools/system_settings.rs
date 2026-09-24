use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub tray_enabled: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tray_enabled: true,
            start_minimized: false,
            close_to_tray: true, // 默认启用关闭时最小化到托盘
        }
    }
}

// 全局托盘状态，保存托盘图标引用和可见状态
pub struct GlobalTrayState {
    pub tray_icon: Mutex<Option<TrayIcon>>,
    pub is_visible: Mutex<bool>,
    pub config: Mutex<AppConfig>,
}

impl GlobalTrayState {
    pub fn new() -> Self {
        let config = load_config().unwrap_or_default();
        Self {
            tray_icon: Mutex::new(None),
            is_visible: Mutex::new(config.tray_enabled),
            config: Mutex::new(config),
        }
    }
}

#[tauri::command]
pub async fn toggle_tray(
    app: AppHandle,
    tray_state: State<'_, GlobalTrayState>,
    enabled: bool,
) -> Result<bool, String> {
    let mut tray_icon = tray_state.tray_icon.lock().map_err(|e| e.to_string())?;
    let mut is_visible = tray_state.is_visible.lock().map_err(|e| e.to_string())?;
    let mut config = tray_state.config.lock().map_err(|e| e.to_string())?;

    if enabled {
        // 启用托盘 - 如果没有托盘，创建一个；如果有托盘，设为可见
        if tray_icon.is_none() {
            let tray = create_tray_icon(&app).map_err(|e| e.to_string())?;
            *tray_icon = Some(tray);
        } else if let Some(ref tray) = *tray_icon {
            tray.set_visible(true).map_err(|e| e.to_string())?;
        }
        *is_visible = true;
        config.tray_enabled = true;
    } else {
        // 禁用托盘 - 设为不可见
        if let Some(ref tray) = *tray_icon {
            tray.set_visible(false).map_err(|e| e.to_string())?;
        }
        *is_visible = false;
        config.tray_enabled = false;
    }

    // 保存配置
    save_config(&config).map_err(|e| e.to_string())?;

    Ok(enabled)
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("devtools")
        .join("devtools-config.json")
}

fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(AppConfig::default())
    }
}

fn save_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(config)?;
    fs::write(config_path, content)?;
    Ok(())
}

#[tauri::command]
pub async fn get_tray_status(tray_state: State<'_, GlobalTrayState>) -> Result<bool, String> {
    let is_visible = tray_state.is_visible.lock().map_err(|e| e.to_string())?;
    Ok(*is_visible)
}

#[tauri::command]
pub async fn set_start_minimized(
    tray_state: State<'_, GlobalTrayState>,
    enabled: bool,
) -> Result<bool, String> {
    let mut config = tray_state.config.lock().map_err(|e| e.to_string())?;
    config.start_minimized = enabled;
    save_config(&config).map_err(|e| e.to_string())?;
    Ok(enabled)
}

#[tauri::command]
pub async fn get_start_minimized_status(
    tray_state: State<'_, GlobalTrayState>,
) -> Result<bool, String> {
    let config = tray_state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.start_minimized)
}

#[tauri::command]
pub async fn set_close_to_tray(
    tray_state: State<'_, GlobalTrayState>,
    enabled: bool,
) -> Result<bool, String> {
    let mut config = tray_state.config.lock().map_err(|e| e.to_string())?;
    config.close_to_tray = enabled;
    save_config(&config).map_err(|e| e.to_string())?;
    Ok(enabled)
}

#[tauri::command]
pub async fn get_close_to_tray_status(
    tray_state: State<'_, GlobalTrayState>,
) -> Result<bool, String> {
    let config = tray_state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.close_to_tray)
}

/// 工具分类(镜像 src/tools/registry.ts 的 toolCategories)
struct ToolCategory {
    id: &'static str,
    name: &'static str,
}

/// 工具条目(镜像 src/tools/registry.ts 的工具定义)
/// 新增工具时需同步本表与 TS registry。
struct ToolEntry {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    width: f64,
    height: f64,
}

/// 8 个工具分类,顺序与 registry.ts 的 toolCategories 一致
fn tray_tool_categories() -> &'static [ToolCategory] {
    &[
        ToolCategory { id: "encoding", name: "编码/解码" },
        ToolCategory { id: "certificate", name: "证书工具" },
        ToolCategory { id: "network", name: "网络工具" },
        ToolCategory { id: "dataformat", name: "数据格式转换" },
        ToolCategory { id: "media", name: "媒体格式转换" },
        ToolCategory { id: "developer", name: "开发工具" },
        ToolCategory { id: "time", name: "时间工具" },
        ToolCategory { id: "settings", name: "设置" },
    ]
}

/// 32 个工具的目录(含窗口尺寸),数据与 registry.ts 一一对应
fn tray_tools() -> &'static [ToolEntry] {
    &[
        // encoding
        ToolEntry { id: "base64converter", name: "Base64 编解码", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "urlencoderdecoder", name: "URL 编解码", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "aescrypto", name: "AES 加密/解密", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "md5crypto", name: "MD5 加密", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "shacrypto", name: "SHA 哈希加密", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "jwtencode", name: "JWT 生成", category: "encoding", width: 1000.0, height: 750.0 },
        ToolEntry { id: "jwtdecode", name: "JWT 解码", category: "encoding", width: 1000.0, height: 750.0 },
        ToolEntry { id: "passwordgenerator", name: "密码生成器", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "passwordhasher", name: "密码加密验证", category: "encoding", width: 900.0, height: 700.0 },
        ToolEntry { id: "rsakeygenerator", name: "RSA 密钥对生成", category: "encoding", width: 900.0, height: 700.0 },
        // certificate
        ToolEntry { id: "certificate", name: "证书查看器", category: "certificate", width: 1100.0, height: 800.0 },
        ToolEntry { id: "csrviewer", name: "CSR 查看", category: "certificate", width: 950.0, height: 750.0 },
        ToolEntry { id: "csrgenerator", name: "CSR 生成", category: "certificate", width: 1000.0, height: 800.0 },
        ToolEntry { id: "pemtopfx", name: "PEM 转 PFX", category: "certificate", width: 950.0, height: 750.0 },
        ToolEntry { id: "pfxtopem", name: "PFX 转 PEM", category: "certificate", width: 950.0, height: 750.0 },
        ToolEntry { id: "sslchecker", name: "在线 SSL 检测", category: "certificate", width: 950.0, height: 750.0 },
        // network
        ToolEntry { id: "subnetcalculator", name: "子网掩码计算器", category: "network", width: 900.0, height: 700.0 },
        ToolEntry { id: "ipinfo", name: "IP 地址信息查询", category: "network", width: 900.0, height: 700.0 },
        ToolEntry { id: "dnsresolver", name: "DNS 解析工具", category: "network", width: 900.0, height: 700.0 },
        ToolEntry { id: "whois", name: "域名 Whois 查询", category: "network", width: 900.0, height: 700.0 },
        // dataformat
        ToolEntry { id: "jsonformatter", name: "JSON 格式化", category: "dataformat", width: 1100.0, height: 800.0 },
        ToolEntry { id: "formatconverter", name: "格式转换器", category: "dataformat", width: 1100.0, height: 800.0 },
        ToolEntry { id: "jsontogo", name: "JSON 转 Go 结构体", category: "dataformat", width: 1100.0, height: 800.0 },
        ToolEntry { id: "sqltogo", name: "SQL 转 Go 结构体", category: "dataformat", width: 1100.0, height: 800.0 },
        ToolEntry { id: "sqltoent", name: "SQL 转 Go Ent ORM", category: "dataformat", width: 1100.0, height: 800.0 },
        // media
        ToolEntry { id: "imageconverter", name: "图片格式转换", category: "media", width: 900.0, height: 700.0 },
        ToolEntry { id: "imagepreview", name: "图片预览器", category: "media", width: 900.0, height: 700.0 },
        ToolEntry { id: "videoconverter", name: "视频格式转换", category: "media", width: 900.0, height: 700.0 },
        // developer
        ToolEntry { id: "regextester", name: "正则表达式测试器", category: "developer", width: 1000.0, height: 750.0 },
        // time
        ToolEntry { id: "timestamp", name: "时间戳转换", category: "time", width: 900.0, height: 700.0 },
        // settings
        ToolEntry { id: "settings", name: "设置", category: "settings", width: 900.0, height: 700.0 },
    ]
}

/// 打开(或聚焦已存在的)工具窗口,逻辑镜像 JS 侧 useToolWindow.openTool。
fn open_tool_window(app: &AppHandle, tool_id: &str) {
    let label = format!("tool-{}", tool_id);

    // 已存在则聚焦复用
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return;
    }

    // 查找工具元数据
    let entry = match tray_tools().iter().find(|t| t.id == tool_id) {
        Some(e) => e,
        None => {
            eprintln!("Tool not found in tray catalog: {}", tool_id);
            return;
        }
    };

    let url = WebviewUrl::App(format!("index.html#/tool/{}", tool_id).into());
    let builder = WebviewWindowBuilder::new(app, &label, url)
        .title(format!("{} - DevTools", entry.name))
        .inner_size(entry.width, entry.height)
        .center()
        .resizable(true)
        .visible(true);

    if let Err(e) = builder.build() {
        eprintln!("Failed to build tool window '{}': {}", tool_id, e);
    }
}

pub fn create_tray_icon(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    // 按分类构建二级子菜单。每个分类的工具 MenuItem 先收集到 Vec(需存活到
    // 对应 Submenu 构建之后),Submenu 收集到 Vec(需存活到 Menu 构建之后)。
    let categories = tray_tool_categories();
    let tools = tray_tools();

    // 收集所有工具 MenuItem(保证生命周期覆盖 Submenu 构建)
    let mut all_tool_items: Vec<Vec<MenuItem<tauri::Wry>>> = Vec::new();
    let mut submenus: Vec<Submenu<tauri::Wry>> = Vec::new();

    for cat in categories {
        let mut cat_items: Vec<MenuItem<tauri::Wry>> = Vec::new();
        for tool in tools.iter().filter(|t| t.category == cat.id) {
            let item = MenuItem::with_id(
                app,
                format!("tool:{}", tool.id),
                tool.name,
                true,
                None::<&str>,
            )?;
            cat_items.push(item);
        }
        // 空分类不创建子菜单(当前所有分类都有工具,此处为防御性处理)
        if cat_items.is_empty() {
            continue;
        }
        let cat_refs: Vec<&dyn IsMenuItem<tauri::Wry>> =
            cat_items.iter().map(|i| i as &dyn IsMenuItem<tauri::Wry>).collect();
        let submenu = Submenu::with_items(app, cat.name, true, &cat_refs)?;
        all_tool_items.push(cat_items);
        submenus.push(submenu);
    }

    let separator = PredefinedMenuItem::separator(app)?;

    // 组装顶层菜单:工具分类子菜单 + 分隔符 + 显示 + 退出
    let mut top_items: Vec<&dyn IsMenuItem<tauri::Wry>> = Vec::new();
    for sub in &submenus {
        top_items.push(sub as &dyn IsMenuItem<tauri::Wry>);
    }
    top_items.push(&separator);
    top_items.push(&show);
    top_items.push(&quit);

    let menu = Menu::with_items(app, &top_items)?;

    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                tool_id if tool_id.starts_with("tool:") => {
                    open_tool_window(app, &tool_id[5..]);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(tray)
}

// 处理窗口关闭事件
pub fn handle_window_close_event(app: &AppHandle) -> bool {
    // 检查托盘状态和关闭到托盘设置
    let tray_state = app.state::<GlobalTrayState>();
    let is_visible = tray_state.is_visible.lock().unwrap();
    let config = tray_state.config.lock().unwrap();

    // 如果托盘可见且启用了关闭到托盘功能，则隐藏窗口而不是退出程序
    // 当托盘禁用时，即使 close_to_tray 为 true，也忽略此设置
    if *is_visible && config.tray_enabled && config.close_to_tray {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        true // 阻止窗口关闭
    } else {
        false // 允许窗口关闭，程序会退出
    }
}
