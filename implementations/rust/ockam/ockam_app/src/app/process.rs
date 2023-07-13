use tauri::{AppHandle, RunEvent, Wry};

/// This is the function dispatching application events
pub fn process_application_event(_app: &AppHandle<Wry>, event: RunEvent) {
    if let RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();
    }
}
