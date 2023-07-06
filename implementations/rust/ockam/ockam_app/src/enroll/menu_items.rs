use tauri::{AppHandle, CustomMenuItem, Wry};

use ockam_api::cli_state::StateDirTrait;
use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::enroll::enroll_user::enroll_user;
use crate::enroll::reset::reset;

pub const ENROLL_MENU_ID: &str = "enroll";
pub const RESET_MENU_ID: &str = "reset";

pub fn menu_items() -> Vec<CustomMenuItem> {
    let options = CommandGlobalOpts::new(GlobalArgs::default());

    let enroll_menu_item = CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e");
    let reset_menu_item = CustomMenuItem::new(RESET_MENU_ID, "Reset...").accelerator("cmd+r");
    if options.state.projects.default().is_ok() {
        vec![enroll_menu_item.disabled(), reset_menu_item]
    } else {
        vec![enroll_menu_item, reset_menu_item.disabled()]
    }
}

/// Enroll the user and show that it has been enrolled
pub fn on_enroll(app: &AppHandle<Wry>) -> tauri::Result<()> {
    enroll_user();
    app.tray_handle()
        .get_item(ENROLL_MENU_ID)
        .set_enabled(false)?;
    app.tray_handle().get_item(RESET_MENU_ID).set_enabled(true)
}

/// Reset the persistent state
pub fn on_reset(app: &AppHandle<Wry>) -> tauri::Result<()> {
    reset();
    app.tray_handle()
        .get_item(ENROLL_MENU_ID)
        .set_enabled(true)?;
    app.tray_handle().get_item(RESET_MENU_ID).set_enabled(false)
}
