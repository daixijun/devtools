use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub tray_enabled: bool,
    pub start_minimized: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tray_enabled: true,
            start_minimized: false,
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

    if enabled {
        // 启用托盘 - 如果没有托盘，创建一个；如果有托盘，设为可见
        if tray_icon.is_none() {
            let tray = create_tray_icon(&app).map_err(|e| e.to_string())?;
            *tray_icon = Some(tray);
        } else if let Some(ref tray) = *tray_icon {
            tray.set_visible(true).map_err(|e| e.to_string())?;
        }
        *is_visible = true;
    } else {
        // 禁用托盘 - 设为不可见
        if let Some(ref tray) = *tray_icon {
            tray.set_visible(false).map_err(|e| e.to_string())?;
        }
        *is_visible = false;
    }

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

pub fn create_tray_icon(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &quit])?;

    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
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
