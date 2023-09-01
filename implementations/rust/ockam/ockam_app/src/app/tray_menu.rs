use tauri::{AppHandle, Manager, State, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Wry};
use tracing::error;

use crate::app::events::SystemTrayOnUpdatePayload;
use crate::app::AppState;
use crate::enroll::{build_enroll_section, build_user_info_section};
use crate::invitations::{self, build_invitations_section};
use crate::local_services::build_local_services_section;
use crate::options::build_options_section;
use crate::remote_services::build_remote_services_section;
use crate::{enroll, local_services, options};

pub async fn build_tray_menu(
    app_handle: &AppHandle,
    payload: Option<SystemTrayOnUpdatePayload>,
) -> SystemTrayMenu {
    let app_state: State<'_, AppState> = app_handle.state();
    let mut tray_menu = SystemTrayMenu::new();
    tray_menu = build_user_info_section(&app_state, tray_menu).await;
    tray_menu = build_enroll_section(&app_state, tray_menu, &payload).await;
    tray_menu = build_local_services_section(&app_state, tray_menu).await;
    tray_menu = build_remote_services_section(app_handle, tray_menu).await;
    tray_menu = build_invitations_section(app_handle, tray_menu).await;
    tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
    tray_menu = build_options_section(&app_state, tray_menu).await;
    tray_menu
}

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(app),
            local_services::SHARED_SERVICE_CREATE_MENU_ID => local_services::on_create(app),
            #[cfg(debug_assertions)]
            options::REFRESH_MENU_ID => options::on_refresh(app),
            options::RESET_MENU_ID => options::on_reset(app),
            options::QUIT_MENU_ID => options::on_quit(),
            id => fallback_for_id(app, id),
        };
        if let Err(e) = result {
            error!("{:?}", e)
        }
    }
}

fn fallback_for_id(app: &AppHandle<Wry>, s: &str) -> tauri::Result<()> {
    if s.starts_with("invitation-") {
        invitations::dispatch_click_event(app, s)
    } else {
        Ok(())
    }
}
