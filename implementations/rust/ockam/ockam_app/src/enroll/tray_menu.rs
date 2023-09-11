use tauri::menu::{IconMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder, NativeIcon};
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::error;

use crate::app::events::SystemTrayOnUpdatePayload;
use crate::app::AppState;
use crate::enroll::enroll_user::enroll_user;

const ENROLL_MENU_EMAIL: &str = "user-email";
const ENROLL_MENU_HEADER_ID: &str = "enroll-header";
const ENROLL_MENU_ID: &str = "enroll";
const ENROLL_MENU_USER_NAME: &str = "user-name";

pub(crate) async fn build_user_info_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if let Ok(user_info) = app_state.user_info().await {
        builder = builder.items(&[
            &IconMenuItemBuilder::new(format!("{} ({})", user_info.name, user_info.nickname))
                .id(ENROLL_MENU_USER_NAME)
                .native_icon(NativeIcon::User)
                .build(app_handle),
            &MenuItemBuilder::new(user_info.email)
                .id(ENROLL_MENU_EMAIL)
                .enabled(false)
                .build(app_handle),
        ])
    }
    builder
}

pub(crate) async fn build_enroll_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
    payload: &Option<SystemTrayOnUpdatePayload>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();

    if let Some(payload) = &payload {
        if let Some(message) = &payload.enroll_status {
            return builder.items(&[
                &MenuItemBuilder::new("Enrolling...")
                    .id(ENROLL_MENU_HEADER_ID)
                    .enabled(false)
                    .build(app_handle),
                &IconMenuItemBuilder::new(message)
                    .id("status")
                    .native_icon(NativeIcon::StatusPartiallyAvailable)
                    .enabled(false)
                    .build(app_handle),
            ]);
        }
    }

    if !app_state.is_enrolled().await.unwrap_or(false) {
        builder = builder.items(&[
            &IconMenuItemBuilder::new("Please enroll")
                .id(ENROLL_MENU_HEADER_ID)
                .native_icon(NativeIcon::User)
                .enabled(false)
                .build(app_handle),
            &MenuItemBuilder::new("Enroll...")
                .id(ENROLL_MENU_ID)
                .accelerator("cmd+e")
                .build(app_handle),
        ])
    }

    builder
}

pub fn process_tray_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &MenuEvent,
) -> tauri::Result<()> {
    match event.id.as_ref() {
        ENROLL_MENU_ID => on_enroll(app),
        _ => Ok(()),
    }
}

/// Event listener for the "Enroll" menu item
/// Enroll the user and show that it has been enrolled
fn on_enroll<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        enroll_user(&app_handle)
            .await
            .map_err(|e| error!(%e, "Failed to enroll user"))
    });
    Ok(())
}
