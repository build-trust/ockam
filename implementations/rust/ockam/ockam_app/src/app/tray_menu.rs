use tauri::{AppHandle, Manager, State, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Wry};
use tracing::error;

use crate::app::AppState;
use crate::enroll::build_enroll_section;
#[cfg(feature = "invitations")]
use crate::invitations::{self, build_invitations_section};
use crate::options::build_options_section;
use crate::shared_service::build_shared_services_section;
use crate::{enroll, options, shared_service};

pub async fn build_tray_menu(app_handle: &AppHandle) -> SystemTrayMenu {
    let app_state: State<'_, AppState> = app_handle.state();
    let mut tray_menu = SystemTrayMenu::new();
    tray_menu = build_enroll_section(&app_state, tray_menu).await;
    tray_menu = build_shared_services_section(&app_state, tray_menu).await;
    #[cfg(feature = "invitations")]
    {
        tray_menu = build_invitations_section(app_handle, tray_menu).await;
    }
    tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
    tray_menu = build_options_section(&app_state, tray_menu).await;
    tray_menu
}

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(app),
            shared_service::SHARED_SERVICE_CREATE_MENU_ID => shared_service::on_create(app),
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

#[cfg(feature = "invitations")]
fn fallback_for_id(app: &AppHandle<Wry>, s: &str) -> tauri::Result<()> {
    if s.starts_with("invitation-") {
        invitations::dispatch_click_event(app, s)
    } else {
        Ok(())
    }
}

#[cfg(not(feature = "invitations"))]
fn fallback_for_id(_app: &AppHandle<Wry>, _s: &str) -> tauri::Result<()> {
    Ok(())
}
