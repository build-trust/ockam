use tauri::Error::Runtime;
use tauri::{AppHandle, CustomMenuItem, Manager, Wry};
use tauri_runtime::Error::SystemTray;
use tracing::info;

use ockam_api::cli_state::StateDirTrait;
use ockam_command::CommandGlobalOpts;

use crate::enroll::enroll_user::enroll_user;
use crate::enroll::reset::reset;

pub const ENROLL_MENU_ID: &str = "enroll";
pub const PLEASE_ENROLL_MENU_ID: &str = "please_enroll";
pub const RESET_MENU_ID: &str = "reset";

#[derive(Clone)]
pub struct EnrollActions {
    pub options: CommandGlobalOpts,
    pub(crate) enroll: CustomMenuItem,
    pub(crate) please_enroll: CustomMenuItem,
    pub(crate) reset: CustomMenuItem,
}

impl EnrollActions {
    pub fn new(options: &CommandGlobalOpts) -> EnrollActions {
        let enroll = CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e");
        let please_enroll =
            CustomMenuItem::new(PLEASE_ENROLL_MENU_ID, "Please Enroll.").accelerator("cmd+e");
        let reset = CustomMenuItem::new(RESET_MENU_ID, "Reset").accelerator("cmd+r");
        match options.state.projects.default() {
            Ok(_) => EnrollActions {
                options: options.clone(),
                enroll: enroll.disabled(),
                please_enroll: please_enroll.disabled(),
                reset,
            },
            Err(_) => {
                info!("There is no default project, please enroll");
                EnrollActions {
                    options: options.clone(),
                    enroll,
                    please_enroll,
                    reset: reset.disabled(),
                }
            }
        }
    }
}

/// Enroll the user and show that it has been enrolled
pub fn on_enroll(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move { enroll_user(&app_handle).await });
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}

/// Reset the persistent state
pub fn on_reset(app: &AppHandle<Wry>) -> tauri::Result<()> {
    match reset(app) {
        Ok(_) => {
            app.tray_handle()
                .get_item(ENROLL_MENU_ID)
                .set_enabled(true)?;
            app.tray_handle().get_item(RESET_MENU_ID).set_enabled(false)
        }
        Err(e) => Err(Runtime(SystemTray(Box::new(e)))),
    }
}
