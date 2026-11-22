//! 应用配置持久化（`dirs::config_dir()/devtools/devtools-config.json`）。
//!
//! 历史版本只写入 `tray_enabled / start_minimized / close_to_tray` 三个字段；
//! 本模块新增字段均带 serde 默认值——旧文件可正常读取，向后兼容。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 主题模式（light/dark/system 三态）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    #[default]
    System,
    Dark,
}

/// 全局快捷键配置（快捷键功能已移除；字段仅为向后兼容保留）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotKeyConfig {
    pub modifier: String,
    pub key: String,
}

impl Default for HotKeyConfig {
    fn default() -> Self {
        Self {
            modifier: if cfg!(target_os = "macos") {
                "option".to_string()
            } else {
                "alt".to_string()
            },
            key: "Space".to_string(),
        }
    }
}

/// `tray_enabled` / `close_to_tray` 的历史默认值（true）。
fn default_true() -> bool {
    true
}

/// `language` 的默认值（跟随系统 locale）。
fn default_language() -> String {
    "system".to_string()
}

/// 应用配置。字段新增必须带 `#[serde(default)]` 以保持与历史版本配置文件双向兼容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_true")]
    pub tray_enabled: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default = "default_language")]
    pub language: String, // "system" | "zh-CN" | "en"（语义见 i18n::Language）
    #[serde(default)]
    pub hotkey: HotKeyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tray_enabled: true,
            start_minimized: false,
            close_to_tray: true, // 默认启用关闭时最小化到托盘
            theme: ThemeMode::default(),
            language: default_language(),
            hotkey: HotKeyConfig::default(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("devtools")
        .join("devtools-config.json")
}

impl AppConfig {
    /// 读取配置；文件不存在时返回默认值。
    ///
    /// 解析失败（文件损坏/字段非法）同样回退默认值，但会打印告警——
    /// 静默吞掉会让用户的主题等设置无声丢失。
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            match fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|content| serde_json::from_str(&content).map_err(|e| e.to_string()))
            {
                Ok(config) => return config,
                Err(err) => eprintln!(
                    "配置文件解析失败，已回退默认配置: {err}（{}）",
                    path.display()
                ),
            }
        }
        Self::default()
    }

    /// 写入配置（父目录不存在则创建）。
    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_compat_with_old_config() {
        // 历史版本写入的配置（只有 3 个字段）必须能被读取
        let old_json = r#"{"tray_enabled":true,"start_minimized":false,"close_to_tray":true}"#;
        let config: AppConfig = serde_json::from_str(old_json).unwrap();
        assert!(config.tray_enabled);
        assert_eq!(config.theme, ThemeMode::System);
        assert_eq!(config.hotkey.modifier, "option");
    }

    #[test]
    fn test_new_fields_serialized() {
        let config = AppConfig {
            theme: ThemeMode::Dark,
            language: "en".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"theme\":\"dark\""));
        assert!(json.contains("\"language\":\"en\""));
    }

    #[test]
    fn test_language_field_backward_compat() {
        // 无 language 字段的旧配置必须按默认值补齐
        let old = r#"{"tray_enabled":true}"#;
        let config: AppConfig = serde_json::from_str(old).unwrap();
        assert_eq!(config.language, "system");
    }

    #[test]
    fn test_partial_config_uses_field_defaults() {
        // 手工编辑/部分写入的配置（缺 bool 字段）不得整体回退默认，
        // theme 等用户设置必须保留；缺的 bool 按历史默认值补齐
        let partial = r#"{"theme":"dark"}"#;
        let config: AppConfig = serde_json::from_str(partial).unwrap();
        assert_eq!(config.theme, ThemeMode::Dark);
        assert!(config.tray_enabled);
        assert!(!config.start_minimized);
        assert!(config.close_to_tray);
    }
}
