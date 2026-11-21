//! 开机自启（macOS LaunchAgent）。
//!
//! 写入 `~/Library/LaunchAgents/<identifier>.plist`，RunAtLoad = true，
//! 启动参数携带 `--autostart`（main.rs 据此隐藏启动）。

use std::path::PathBuf;

/// bundle identifier（沿用历史版本，保证已写入的 LaunchAgent plist 互认）。
const IDENTIFIER: &str = "devtools.linuxyunwei.com";

fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{}.plist", IDENTIFIER))
}

/// 当前是否已启用开机自启（plist 存在即视为启用）。
pub fn status() -> bool {
    plist_path().exists()
}

/// 启用/禁用开机自启。
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
        <string>--autostart</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#);
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
