use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, Wry};
#[cfg(target_os = "macos")]
use tauri_runtime::menu::NativeImage;

use crate::app::AppState;
use crate::enroll::enroll_user::enroll_user;

pub const ENROLL_MENU_EMAIL: &str = "user-email";
pub const ENROLL_MENU_HEADER_ID: &str = "enroll-header";
pub const ENROLL_MENU_ID: &str = "enroll";
pub const ENROLL_MENU_USER_NAME: &str = "user-name";

pub(crate) async fn build_enroll_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    if app_state.is_enrolled().await.unwrap_or(false) {
        match app_state.model(|m| m.get_user_info()).await {
            Some(user_info) => {
                let item = CustomMenuItem::new(
                    ENROLL_MENU_USER_NAME,
                    format!("{} ({})", user_info.name, user_info.nickname),
                );
                #[cfg(target_os = "macos")]
                let item = item.native_image(NativeImage::User);
                tray_menu
                    .add_item(item)
                    .add_item(CustomMenuItem::new(ENROLL_MENU_EMAIL, user_info.email).disabled())
            }
            None => tray_menu,
        }
    } else {
        tray_menu
            .add_item(CustomMenuItem::new(ENROLL_MENU_HEADER_ID, "Please enroll").disabled())
            .add_item(CustomMenuItem::new(ENROLL_MENU_ID, "Enroll...").accelerator("cmd+e"))
    }
}

/// Event listener for the "Enroll" menu item
/// Enroll the user and show that it has been enrolled
pub fn on_enroll(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move { enroll_user(&app_handle).await });
    Ok(())
}
