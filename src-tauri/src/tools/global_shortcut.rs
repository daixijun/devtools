use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotKeyConfig {
    pub modifier: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalShortcutConfig {
    pub hotkey: HotKeyConfig,
    pub enabled: bool,
}

impl Default for GlobalShortcutConfig {
    fn default() -> Self {
        Self {
            hotkey: HotKeyConfig {
                modifier: if cfg!(target_os = "macos") {
                    "option".to_string()
                } else {
                    "alt".to_string()
                },
                key: "Space".to_string(),
            },
            enabled: true,
        }
    }
}

pub struct GlobalShortcutState {
    pub config: Arc<Mutex<GlobalShortcutConfig>>,
    pub current_shortcut: Arc<Mutex<Option<String>>>,
}

impl GlobalShortcutState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(GlobalShortcutConfig::default())),
            current_shortcut: Arc::new(Mutex::new(None)),
        }
    }
}

fn hotkey_config_to_accelerator(config: &HotKeyConfig) -> Result<String, String> {
    let modifier = match config.modifier.as_str() {
        "alt" | "option" => {
            if cfg!(target_os = "macos") {
                "Option"
            } else {
                "Alt"
            }
        }
        "ctrl" => "Ctrl",
        "cmd" => {
            if cfg!(target_os = "macos") {
                "Cmd"
            } else {
                "Ctrl" // Windows/Linux 上用 Ctrl 代替 Cmd
            }
        }
        "shift" => "Shift",
        _ => return Err("不支持的修饰键".to_string()),
    };

    let key = match config.key.as_str() {
        "Space" => "Space",
        k if k.len() == 1 => &k.to_uppercase(),
        _ => return Err("不支持的按键".to_string()),
    };

    Ok(format!("{}+{}", modifier, key))
}

fn toggle_window_visibility(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(true) => {
                let _ = window.hide();
            }
            Ok(false) => {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Err(_) => {
                // 如果无法获取状态，尝试显示窗口
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}

#[tauri::command]
pub fn register_global_shortcut(
    _app: AppHandle,
    config: HotKeyConfig,
    state: State<'_, GlobalShortcutState>,
) -> Result<bool, String> {
    // 验证快捷键配置
    if let Err(e) = hotkey_config_to_accelerator(&config) {
        return Err(e);
    }

    // 更新配置状态
    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.hotkey = config;
        config_guard.enabled = true;
    }

    Ok(true)
}

#[tauri::command]
pub fn unregister_global_shortcut(
    _app: AppHandle,
    state: State<'_, GlobalShortcutState>,
) -> Result<bool, String> {
    // 更新配置状态
    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.enabled = false;
    }

    Ok(true)
}

#[tauri::command]
pub fn get_global_shortcut_config(
    state: State<'_, GlobalShortcutState>,
) -> Result<GlobalShortcutConfig, String> {
    let config = state.config.lock().unwrap().clone();
    Ok(config)
}

#[tauri::command]
pub fn set_global_shortcut_enabled(
    _app: AppHandle,
    enabled: bool,
    state: State<'_, GlobalShortcutState>,
) -> Result<bool, String> {
    // 更新配置状态
    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.enabled = enabled;
    }

    Ok(true)
}

#[tauri::command]
pub async fn handle_global_shortcut_triggered(app: AppHandle) -> Result<(), String> {
    toggle_window_visibility(&app);
    Ok(())
}

pub fn initialize_global_shortcut(
    _app: &AppHandle,
    state: &GlobalShortcutState,
) -> Result<(), String> {
    // 初始化时验证配置，但不发送事件
    // 前端会在启动时主动获取配置并注册快捷键
    let config = {
        let config_guard = state.config.lock().unwrap();
        config_guard.clone()
    };

    if config.enabled {
        // 验证快捷键配置是否有效
        hotkey_config_to_accelerator(&config.hotkey)?;
    }

    Ok(())
}
