use tauri::menu::{IconMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder, NativeIcon};
use tauri::{AppHandle, Icon, Manager, Runtime, State};
use tracing::error;

use crate::app::events::SystemTrayOnUpdatePayload;
use crate::app::AppState;
use crate::enroll::enroll_user::enroll_user;
use crate::icons::themed_icon;

const ENROLL_MENU_ID: &str = "enroll";

pub(crate) async fn build_user_info_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if !app_state.is_enrolled().await.unwrap_or(false) {
        return builder;
    }
    if let Ok(user_info) = app_state.user_info().await {
        builder = builder
            .items(&[
                &MenuItemBuilder::new(format!("{} ({})", user_info.name, user_info.nickname))
                    .enabled(false)
                    .build(app_handle),
                &MenuItemBuilder::new(user_info.email)
                    .enabled(false)
                    .build(app_handle),
            ])
            .separator()
    }
    builder
}

pub(crate) async fn build_enroll_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
    payload: Option<&SystemTrayOnUpdatePayload>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<AppState> = app_handle.state();
    if let Some(message) = payload.and_then(|p| p.enroll_status.as_ref()) {
        return builder
            .items(&[&IconMenuItemBuilder::new(message)
                .id("status")
                .native_icon(NativeIcon::StatusPartiallyAvailable)
                .enabled(false)
                .build(app_handle)])
            .separator();
    }
    if !app_state.is_enrolled().await.unwrap_or(false) {
        builder = builder
            .items(&[
                &IconMenuItemBuilder::new("Start by enrolling your computer")
                    .enabled(false)
                    .build(app_handle),
                &IconMenuItemBuilder::new("Enroll")
                    .id(ENROLL_MENU_ID)
                    .icon(Icon::Raw(themed_icon("box-arrow-in-right")))
                    .accelerator("cmd+e")
                    .build(app_handle),
            ])
            .separator()
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
        let _ = enroll_user(&app_handle)
            .await
            .map_err(|e| error!(%e, "Failed to enroll user"));
    });
    Ok(())
}
