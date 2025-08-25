use crate::utils::error::DevToolError;
use crate::utils::response::DevToolResponse;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

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

fn hotkey_config_to_accelerator(config: &HotKeyConfig) -> Result<String, DevToolError> {
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
        _ => return Err(DevToolError::ValidationError("不支持的修饰键".to_string())),
    };

    let key = match config.key.as_str() {
        "Space" => "Space",
        k if k.len() == 1 => &k.to_uppercase(),
        _ => return Err(DevToolError::ValidationError("不支持的按键".to_string())),
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
pub async fn register_global_shortcut(
    app: AppHandle,
    config: HotKeyConfig,
    state: State<'_, GlobalShortcutState>,
) -> Result<DevToolResponse<bool>, DevToolError> {
    let accelerator = hotkey_config_to_accelerator(&config)?;

    // 先取消注册旧的快捷键
    if let Ok(current) = state.current_shortcut.lock() {
        if let Some(old_shortcut) = current.as_ref() {
            // 使用前端 API 取消注册
            if let Err(e) = app.emit("unregister-shortcut", old_shortcut) {
                eprintln!("Failed to emit unregister event: {}", e);
            }
        }
    }

    // 发送注册事件到前端
    if let Err(e) = app.emit("register-shortcut", &accelerator) {
        return Err(DevToolError::SystemError(format!(
            "发送注册事件失败: {}",
            e
        )));
    }

    // 更新状态
    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.hotkey = config;
        config_guard.enabled = true;
    }
    {
        let mut current = state.current_shortcut.lock().unwrap();
        *current = Some(accelerator);
    }

    Ok(DevToolResponse::success(true))
}

#[tauri::command]
pub async fn unregister_global_shortcut(
    app: AppHandle,
    state: State<'_, GlobalShortcutState>,
) -> Result<DevToolResponse<bool>, DevToolError> {
    if let Ok(current) = state.current_shortcut.lock() {
        if let Some(shortcut) = current.as_ref() {
            // 发送取消注册事件到前端
            if let Err(e) = app.emit("unregister-shortcut", shortcut) {
                eprintln!("Failed to emit unregister event: {}", e);
            }
        }
    }

    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.enabled = false;
    }
    {
        let mut current = state.current_shortcut.lock().unwrap();
        *current = None;
    }

    Ok(DevToolResponse::success(true))
}

#[tauri::command]
pub async fn get_global_shortcut_config(
    state: State<'_, GlobalShortcutState>,
) -> Result<DevToolResponse<GlobalShortcutConfig>, DevToolError> {
    let config = state.config.lock().unwrap().clone();
    Ok(DevToolResponse::success(config))
}

#[tauri::command]
pub async fn set_global_shortcut_enabled(
    app: AppHandle,
    enabled: bool,
    state: State<'_, GlobalShortcutState>,
) -> Result<DevToolResponse<bool>, DevToolError> {
    if enabled {
        let config = {
            let config_guard = state.config.lock().unwrap();
            config_guard.hotkey.clone()
        };
        register_global_shortcut(app, config, state).await
    } else {
        unregister_global_shortcut(app, state).await
    }
}

#[tauri::command]
pub async fn handle_global_shortcut_triggered(app: AppHandle) -> Result<(), DevToolError> {
    toggle_window_visibility(&app);
    Ok(())
}

pub fn initialize_global_shortcut(
    app: &AppHandle,
    state: &GlobalShortcutState,
) -> Result<(), DevToolError> {
    let config = {
        let config_guard = state.config.lock().unwrap();
        config_guard.clone()
    };

    if config.enabled {
        let accelerator = hotkey_config_to_accelerator(&config.hotkey)?;

        // 延迟发送初始化事件，避免时序问题
        let app_handle = app.clone();
        let accelerator_clone = accelerator.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = app_handle.emit("init-shortcut", &accelerator_clone);
        });

        let mut current = state.current_shortcut.lock().unwrap();
        *current = Some(accelerator);
    }

    Ok(())
}
