use tauri::{AppHandle, RunEvent, SystemTrayEvent, Wry};

use crate::{enroll, quit};

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(app).unwrap(),
            quit::QUIT_MENU_ID => quit::on_quit(app).unwrap(),
            _ => {}
        }
    }
}

/// This is the function dispatching application events
pub fn process_application_event(_app: &AppHandle<Wry>, event: RunEvent) {
    if let RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();
    }
}
