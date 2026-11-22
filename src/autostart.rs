//! 开机自启（跨平台）。
//!
//! 各平台机制：macOS 写 LaunchAgent plist（RunAtLoad）；Windows 写
//! `HKCU\...\Run` 注册表值；Linux 写 XDG 自启 desktop 文件（`~/.config/
//! autostart/`）。启动参数统一携带 `--autostart`（main.rs 据此隐藏启动）。

/// 启动参数：自启入口与手动启动的唯一区分标记。
const AUTOSTART_ARG: &str = "--autostart";

// ---------------------------------------------------------------------------
// macOS：LaunchAgent plist（~/Library/LaunchAgents/<identifier>.plist）
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::AUTOSTART_ARG;
    use std::path::PathBuf;

    /// bundle identifier（沿用历史版本，保证已写入的 LaunchAgent plist 互认）。
    const IDENTIFIER: &str = "devtools.linuxyunwei.com";

    fn plist_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{IDENTIFIER}.plist"))
    }

    pub fn status() -> bool {
        plist_path().exists()
    }

    pub fn set(enabled: bool) -> std::io::Result<()> {
        let path = plist_path();
        if enabled {
            let exe = std::env::current_exe()?.display().to_string();
            let content = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{IDENTIFIER}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>{AUTOSTART_ARG}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#
            );
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)
        } else if path.exists() {
            std::fs::remove_file(path)
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Windows：注册表 Run 键（HKCU，无需管理员权限）
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod imp {
    use super::AUTOSTART_ARG;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;

    /// HKCU Run 键路径与值名（值名全局唯一标识本应用）。
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "DevTools";

    pub fn status() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey(RUN_KEY)
            .and_then(|key| key.get_value::<String, _>(VALUE_NAME))
            .is_ok()
    }

    pub fn set(enabled: bool) -> std::io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE)?;
        if enabled {
            // 带引号的路径：防 Program Files 等含空格路径被拆参
            let cmd = format!("\"{}\" {AUTOSTART_ARG}", std::env::current_exe()?.display());
            key.set_value(VALUE_NAME, &cmd)
        } else {
            match key.delete_value(VALUE_NAME) {
                Ok(()) => Ok(()),
                // 值不存在即视为已禁用
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Linux：XDG 自启 desktop 文件（~/.config/autostart/）
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod imp {
    use super::AUTOSTART_ARG;
    use std::path::PathBuf;

    fn desktop_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("autostart")
            .join("devtools.desktop")
    }

    pub fn status() -> bool {
        desktop_path().exists()
    }

    pub fn set(enabled: bool) -> std::io::Result<()> {
        let path = desktop_path();
        if enabled {
            let exe = std::env::current_exe()?.display().to_string();
            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=DevTools\n\
                 Exec="{exe}" {AUTOSTART_ARG}\n\
                 X-GNOME-Autostart-enabled=true\n"
            );
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)
        } else if path.exists() {
            std::fs::remove_file(path)
        } else {
            Ok(())
        }
    }
}

/// 当前是否已启用开机自启（平台自启标记存在即视为启用）。
pub fn status() -> bool {
    imp::status()
}

/// 启用/禁用开机自启。
pub fn set(enabled: bool) -> std::io::Result<()> {
    imp::set(enabled)
}
