use tauri::{AppHandle, Manager, RunEvent, Wry};

/// This is the function dispatching application events
pub fn process_application_event(app: &AppHandle<Wry>, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Ready => {
            app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
        }
        _ => {}
    }
}
