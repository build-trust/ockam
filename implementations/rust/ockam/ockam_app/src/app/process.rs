use tauri::{AppHandle, RunEvent, SystemTrayEvent, Wry};
use tracing::error;

use crate::ctx::TauriCtx;
use crate::enroll::DefaultBackend;
use crate::{enroll, quit, tcp};

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    let ctx = TauriCtx::new(app.clone());
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(DefaultBackend, ctx),
            tcp::outlet::TCP_OUTLET_CREATE_MENU_ID => tcp::outlet::on_create(ctx),
            enroll::RESET_MENU_ID => enroll::on_reset(DefaultBackend, ctx),
            quit::QUIT_MENU_ID => quit::on_quit(ctx),
            _ => Ok(()),
        };
        if let Err(e) = result {
            error!("{:?}", e)
        }
    }
}

/// This is the function dispatching application events
pub fn process_application_event(_app: &AppHandle<Wry>, event: RunEvent) {
    if let RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();
    }
}
