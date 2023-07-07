use tauri::{AppHandle, CustomMenuItem, Wry};
use tracing::info;

use ockam_api::cli_state::StateDirTrait;
use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::enroll::backend::Backend;

pub const ENROLL_MENU_ID: &str = "enroll";
pub const RESET_MENU_ID: &str = "reset";

#[derive(Clone)]
pub struct EnrollActions {
    pub options: CommandGlobalOpts,
    pub(crate) enroll: CustomMenuItem,
    pub(crate) reset: CustomMenuItem,
}

impl EnrollActions {
    pub fn new() -> EnrollActions {
        let enroll = CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e");
        let reset = CustomMenuItem::new(RESET_MENU_ID, "Reset...").accelerator("cmd+r");
        let options = CommandGlobalOpts::new(GlobalArgs::default());
        match options.state.projects.default() {
            Ok(_) => EnrollActions {
                options,
                enroll: enroll.disabled(),
                reset,
            },
            Err(_) => {
                info!("There is no default project, please enroll");
                EnrollActions {
                    options,
                    enroll,
                    reset: reset.disabled(),
                }
            }
        }
    }
}

/// Enroll the user and show that it has been enrolled
pub fn on_enroll(backend: impl Backend, app: &AppHandle<Wry>) -> tauri::Result<()> {
    if backend.enroll_user().is_ok() {
        app.tray_handle()
            .get_item(ENROLL_MENU_ID)
            .set_enabled(false)?;
        app.tray_handle().get_item(RESET_MENU_ID).set_enabled(true)
    } else {
        Ok(())
    }
}

/// Reset the persistent state
pub fn on_reset(backend: impl Backend, app: &AppHandle<Wry>) -> tauri::Result<()> {
    if backend.reset().is_ok() {
        app.tray_handle()
            .get_item(ENROLL_MENU_ID)
            .set_enabled(true)?;
        app.tray_handle().get_item(RESET_MENU_ID).set_enabled(false)
    } else {
        Ok(())
    }
}
