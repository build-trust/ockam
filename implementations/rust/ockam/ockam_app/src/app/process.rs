use crate::app::events::system_tray_on_update;
use tauri::{AppHandle, RunEvent, Runtime};

/// This is the function dispatching application events
pub fn process_application_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Ready => {
            system_tray_on_update(app);
        }
        _ => {}
    }
}
