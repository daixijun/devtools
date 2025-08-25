pub mod tools;
mod utils;

use tauri::Manager;
use tools::system_settings::GlobalTrayState;
use tools::global_shortcut::GlobalShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(GlobalTrayState::new())
        .manage(GlobalShortcutState::new())
        .invoke_handler(tauri::generate_handler![
            tools::autostart::set_autostart,
            tools::autostart::get_autostart_status,
            tools::certificate_converter::convert_pfx_to_pem,
            tools::certificate_converter::convert_pem_to_pfx,
            tools::certificate_viewer::parse_pem_certificate,
            tools::certificate_viewer::parse_pfx_certificate,
            tools::global_shortcut::register_global_shortcut,
            tools::global_shortcut::unregister_global_shortcut,
            tools::global_shortcut::get_global_shortcut_config,
            tools::global_shortcut::set_global_shortcut_enabled,
            tools::global_shortcut::handle_global_shortcut_triggered,
            tools::ip_info::query_ip_info,
            tools::json_to_go::convert_json_to_go,
            tools::regex_tester::test_regex,
            tools::regex_tester::replace_regex,
            tools::regex_tester::validate_regex,
            tools::sql_to_go::convert_sql_to_go,
            tools::sql_to_ent::convert_sql_to_ent,
            tools::ssl_checker::check_ssl_info,
            tools::system_settings::toggle_tray,
            tools::system_settings::get_tray_status,
            tools::system_settings::set_start_minimized,
            tools::system_settings::get_start_minimized_status
        ])
        .setup(|app| {
            let tray_state = app.state::<GlobalTrayState>();
            let shortcut_state = app.state::<GlobalShortcutState>();
            
            // 检查是否启用托盘
            let should_show_tray = {
                let config = tray_state.config.lock().unwrap();
                config.tray_enabled
            };
            
            if should_show_tray {
                if let Ok(tray) = tools::system_settings::create_tray_icon(app.handle()) {
                    if let Ok(mut tray_icon) = tray_state.tray_icon.lock() {
                        *tray_icon = Some(tray);
                    }
                }
            }
            
            // 初始化全局快捷键
            if let Err(e) = tools::global_shortcut::initialize_global_shortcut(app.handle(), &shortcut_state) {
                eprintln!("Failed to initialize global shortcut: {}", e);
            }
            
            // 检查是否为自启动模式，如果是且启用了启动时最小化，则隐藏窗口
            let args: Vec<String> = std::env::args().collect();
            let is_autostart = args.contains(&"--autostart".to_string());
            
            if is_autostart {
                let should_start_minimized = {
                    let config = tray_state.config.lock().unwrap();
                    config.start_minimized
                };
                
                if should_start_minimized {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
