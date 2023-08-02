use crate::shared_service::SHARED_SERVICE_WINDOW_ID;
use tauri::{AppHandle, Manager, Wry};

#[tauri::command]
pub fn tcp_outlet_close_window(app: AppHandle<Wry>) -> Result<(), String> {
    let _ = app.get_window(SHARED_SERVICE_WINDOW_ID).map(|w| w.close());
    Ok(())
}
