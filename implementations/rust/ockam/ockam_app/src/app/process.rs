use crate::app::events::system_tray_on_update;
use tauri::{AppHandle, RunEvent, Runtime};

/// This is the function dispatching application events
pub fn process_application_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    match event {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        RunEvent::Opened { urls } => {
            urls.into_iter().for_each(|url| {
                use tauri::Manager;
                app.trigger_global(crate::ockam_url::events::URL_OPENED, Some(url.into()));
            });
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Ready => {
            system_tray_on_update(app);
        }
        _ => {}
    }
}
