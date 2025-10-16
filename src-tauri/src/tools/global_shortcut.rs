use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutWrapper};

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
    pub current_shortcut: Arc<Mutex<Option<Shortcut>>>,
}

impl GlobalShortcutState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(GlobalShortcutConfig::default())),
            current_shortcut: Arc::new(Mutex::new(None)),
        }
    }
}

fn parse_key_code(key: &str) -> Result<Code, String> {
    match key.to_lowercase().as_str() {
        "space" => Ok(Code::Space),
        "a" => Ok(Code::KeyA),
        "b" => Ok(Code::KeyB),
        "c" => Ok(Code::KeyC),
        "d" => Ok(Code::KeyD),
        "e" => Ok(Code::KeyE),
        "f" => Ok(Code::KeyF),
        "g" => Ok(Code::KeyG),
        "h" => Ok(Code::KeyH),
        "i" => Ok(Code::KeyI),
        "j" => Ok(Code::KeyJ),
        "k" => Ok(Code::KeyK),
        "l" => Ok(Code::KeyL),
        "m" => Ok(Code::KeyM),
        "n" => Ok(Code::KeyN),
        "o" => Ok(Code::KeyO),
        "p" => Ok(Code::KeyP),
        "q" => Ok(Code::KeyQ),
        "r" => Ok(Code::KeyR),
        "s" => Ok(Code::KeyS),
        "t" => Ok(Code::KeyT),
        "u" => Ok(Code::KeyU),
        "v" => Ok(Code::KeyV),
        "w" => Ok(Code::KeyW),
        "x" => Ok(Code::KeyX),
        "y" => Ok(Code::KeyY),
        "z" => Ok(Code::KeyZ),
        "0" => Ok(Code::Digit0),
        "1" => Ok(Code::Digit1),
        "2" => Ok(Code::Digit2),
        "3" => Ok(Code::Digit3),
        "4" => Ok(Code::Digit4),
        "5" => Ok(Code::Digit5),
        "6" => Ok(Code::Digit6),
        "7" => Ok(Code::Digit7),
        "8" => Ok(Code::Digit8),
        "9" => Ok(Code::Digit9),
        "f1" => Ok(Code::F1),
        "f2" => Ok(Code::F2),
        "f3" => Ok(Code::F3),
        "f4" => Ok(Code::F4),
        "f5" => Ok(Code::F5),
        "f6" => Ok(Code::F6),
        "f7" => Ok(Code::F7),
        "f8" => Ok(Code::F8),
        "f9" => Ok(Code::F9),
        "f10" => Ok(Code::F10),
        "f11" => Ok(Code::F11),
        "f12" => Ok(Code::F12),
        _ => Err(format!("Unsupported key: {}", key)),
    }
}

fn parse_modifier(modifier: &str) -> Modifiers {
    match modifier.to_lowercase().as_str() {
        "alt" | "option" => Modifiers::ALT,
        "ctrl" | "control" => Modifiers::CONTROL,
        "cmd" | "command" => {
            if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            }
        }
        "shift" => Modifiers::SHIFT,
        _ => Modifiers::empty(),
    }
}

fn create_shortcut(config: &HotKeyConfig) -> Result<Shortcut, String> {
    let modifiers = parse_modifier(&config.modifier);
    let key = parse_key_code(&config.key)?;

    Ok(Shortcut::new(Some(modifiers), key))
}

pub fn toggle_window_visibility(app: &AppHandle) {
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
    app: AppHandle,
    config: HotKeyConfig,
    state: State<'_, GlobalShortcutState>,
) -> Result<bool, String> {
    // 先取消已注册的快捷键
    {
        let mut current_shortcut = state.current_shortcut.lock().unwrap();
        if let Some(ref shortcut) = *current_shortcut {
            let shortcut_wrapper: ShortcutWrapper = shortcut
                .clone()
                .try_into()
                .map_err(|e| format!("Failed to convert shortcut: {:?}", e))?;
            if let Err(e) = app.global_shortcut().unregister(shortcut_wrapper) {
                eprintln!("Failed to unregister existing shortcut: {}", e);
            }
        }
        *current_shortcut = None;
    }

    // 创建并注册新的快捷键
    let shortcut = create_shortcut(&config)?;

    if let Err(e) = app.global_shortcut().register(shortcut) {
        return Err(format!("Failed to register global shortcut: {}", e));
    }

    // 更新配置状态
    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.hotkey = config;
        config_guard.enabled = true;
    }

    // 更新当前注册的快捷键
    {
        let mut current_shortcut = state.current_shortcut.lock().unwrap();
        *current_shortcut = Some(shortcut);
    }

    Ok(true)
}

#[tauri::command]
pub fn unregister_global_shortcut(
    app: AppHandle,
    state: State<'_, GlobalShortcutState>,
) -> Result<bool, String> {
    // 取消已注册的快捷键
    {
        let mut current_shortcut = state.current_shortcut.lock().unwrap();
        if let Some(ref shortcut) = *current_shortcut {
            let shortcut_wrapper: ShortcutWrapper = shortcut
                .clone()
                .try_into()
                .map_err(|e| format!("Failed to convert shortcut: {:?}", e))?;
            if let Err(e) = app.global_shortcut().unregister(shortcut_wrapper) {
                return Err(format!("Failed to unregister global shortcut: {}", e));
            }
        }
        *current_shortcut = None;
    }

    // 更新配置状态
    {
        let mut config_guard = state.config.lock().unwrap();
        config_guard.enabled = false;
    }

    // 清除当前注册的快捷键
    {
        let mut current_shortcut = state.current_shortcut.lock().unwrap();
        *current_shortcut = None;
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

pub fn handle_global_shortcut_triggered(
    app: &AppHandle,
    _shortcut: &Shortcut,
    _event: &tauri_plugin_global_shortcut::ShortcutEvent,
) {
    toggle_window_visibility(app);
}

pub fn initialize_global_shortcut(
    app: &AppHandle,
    state: &GlobalShortcutState,
) -> Result<(), String> {
    // 获取配置
    let config = {
        let config_guard = state.config.lock().unwrap();
        config_guard.clone()
    };

    if config.enabled {
        // 创建并注册快捷键
        let shortcut = create_shortcut(&config.hotkey)?;

        if let Err(e) = app.global_shortcut().register(shortcut) {
            return Err(format!("Failed to register global shortcut: {}", e));
        }

        println!(
            "Global shortcut registered: {}+{}",
            config.hotkey.modifier, config.hotkey.key
        );

        // 更新当前注册的快捷键
        {
            let mut current_shortcut = state.current_shortcut.lock().unwrap();
            *current_shortcut = Some(shortcut);
        }
    }

    Ok(())
}
