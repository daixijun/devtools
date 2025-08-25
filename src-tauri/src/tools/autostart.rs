use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub async fn set_autostart<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<bool, String> {
    let autostart = app.autolaunch();

    if enabled {
        autostart.enable().map_err(|e| e.to_string())?;
    } else {
        autostart.disable().map_err(|e| e.to_string())?;
    }

    Ok(enabled)
}

#[tauri::command]
pub async fn get_autostart_status<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let autostart = app.autolaunch();
    autostart.is_enabled().map_err(|e| e.to_string())
}
