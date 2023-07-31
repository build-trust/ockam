use tauri::{AppHandle, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, Wry};
use tracing::error;

use crate::app::AppState;
use crate::enroll::build_enroll_section;
#[cfg(feature = "invitations")]
use crate::invitations::build_invitations_section;
use crate::options::build_options_section;
use crate::shared_service::build_shared_services_section;
use crate::{enroll, options, shared_service};

pub async fn build_tray_menu(app_state: &AppState) -> SystemTrayMenu {
    let mut tray_menu = SystemTrayMenu::new();
    tray_menu = build_enroll_section(app_state, tray_menu).await;
    tray_menu = build_shared_services_section(app_state, tray_menu).await;
    #[cfg(feature = "invitations")]
    {
        tray_menu = build_invitations_section(app_state, tray_menu).await;
    }
    tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
    tray_menu = build_options_section(app_state, tray_menu).await;
    tray_menu
}

/// This is the function dispatching events for the SystemTray
pub fn process_system_tray_event(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    if let SystemTrayEvent::MenuItemClick { id, .. } = event {
        let result = match id.as_str() {
            enroll::ENROLL_MENU_ID => enroll::on_enroll(app),
            shared_service::SHARED_SERVICE_CREATE_MENU_ID => shared_service::on_create(app),
            options::RESET_MENU_ID => options::on_reset(app),
            options::QUIT_MENU_ID => options::on_quit(),
            _ => Ok(()),
        };
        if let Err(e) = result {
            error!("{:?}", e)
        }
    }
}
