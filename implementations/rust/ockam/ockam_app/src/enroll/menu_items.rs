use tauri::CustomMenuItem;
use tauri::Error::Runtime;
use tauri_runtime::Error::SystemTray;
use tracing::info;

use ockam_api::cli_state::StateDirTrait;
use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::ctx::TauriCtx;
use crate::enroll::enroll_user::enroll_user;
use crate::enroll::reset::reset;

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
        let reset = CustomMenuItem::new(RESET_MENU_ID, "Reset").accelerator("cmd+r");
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
pub fn on_enroll(ctx: TauriCtx) -> tauri::Result<()> {
    match enroll_user() {
        Ok(_) => {
            ctx.app_handle()
                .tray_handle()
                .get_item(ENROLL_MENU_ID)
                .set_enabled(false)?;
            ctx.app_handle()
                .tray_handle()
                .get_item(RESET_MENU_ID)
                .set_enabled(true)
        }
        Err(e) => Err(Runtime(SystemTray(Box::new(e)))),
    }
}

/// Reset the persistent state
pub fn on_reset(ctx: TauriCtx) -> tauri::Result<()> {
    match reset(&ctx) {
        Ok(_) => {
            ctx.app_handle()
                .tray_handle()
                .get_item(ENROLL_MENU_ID)
                .set_enabled(true)?;
            ctx.app_handle()
                .tray_handle()
                .get_item(RESET_MENU_ID)
                .set_enabled(false)
        }
        Err(e) => Err(Runtime(SystemTray(Box::new(e)))),
    }
}
