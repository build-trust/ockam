use crate::app::AppState;
use tauri::menu::{IconMenuItem, MenuBuilder, MenuItem, NativeIcon};
use tauri::{AppHandle, Manager, Runtime, State};
#[cfg(target_os = "macos")]
use tauri_runtime::menu::NativeImage;

use crate::enroll::enroll_user::enroll_user;

pub const ENROLL_MENU_HEADER_ID: &str = "enroll-header";
pub const ENROLL_MENU_ID: &str = "enroll";
pub const ENROLL_MENU_USER_NAME: &str = "user-name";

pub(crate) async fn build_enroll_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if app_state.is_enrolled().await {
        if let Some(user_info) = app_state.model(|m| m.get_user_info()).await {
            builder = builder.item(&IconMenuItem::with_id_and_native_icon(
                app_handle,
                ENROLL_MENU_USER_NAME,
                format!("{} ({})", user_info.name, user_info.nickname),
                true,
                Some(NativeIcon::User),
                None,
            ));
        }
    } else {
        builder = builder.items(&[
            &MenuItem::with_id(
                app_handle,
                ENROLL_MENU_HEADER_ID,
                "Please enroll",
                false,
                None,
            ),
            &MenuItem::with_id(app_handle, ENROLL_MENU_ID, "Enroll...", true, Some("cmd+e")),
        ]);
    }

    builder
}

/// Event listener for the "Enroll" menu item
/// Enroll the user and show that it has been enrolled
pub fn on_enroll<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move { enroll_user(&app_handle).await });
    Ok(())
}
