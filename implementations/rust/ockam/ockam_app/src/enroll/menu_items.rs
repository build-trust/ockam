use tauri::{AppHandle, CustomMenuItem, Wry};

use crate::enroll::enroll_user::enroll_user;

pub const ENROLL_MENU_ID: &str = "enroll";

pub fn menu_items() -> Vec<CustomMenuItem> {
    vec![CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e")]
}

/// Enroll the user and show that it has been enrolled
pub fn on_enroll(app: &AppHandle<Wry>) -> tauri::Result<()> {
    enroll_user();
    app.tray_handle().get_item("enroll").set_title("Enrolled")
}
