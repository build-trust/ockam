use tauri::{AppHandle, CustomMenuItem, Manager, SystemTrayMenu, SystemTraySubmenu, Wry};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_runtime::menu::SystemTrayMenuItem;

use ockam_api::nodes::models::portal::ServiceStatus;

use crate::app::AppState;

pub const SHARED_SERVICE_CREATE_MENU_ID: &str = "shared_service_create";
pub const SHARED_SERVICE_WINDOW_ID: &str = "shared_service_creation";

pub(crate) async fn build_local_services_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    if !app_state.is_enrolled().await.unwrap_or(false) {
        return tray_menu;
    };

    let mut local_services_submenu = SystemTrayMenu::new().add_item(CustomMenuItem::new(
        SHARED_SERVICE_CREATE_MENU_ID,
        "Create...",
    ));
    local_services_submenu = app_state
        .tcp_outlet_list()
        .await
        .iter()
        .map(service_status_submenu)
        .fold(local_services_submenu, |menu, submenu| {
            menu.add_submenu(submenu)
        });

    tray_menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_submenu(SystemTraySubmenu::new(
            "Local services",
            local_services_submenu,
        ))
}

fn service_status_submenu(service: &ServiceStatus) -> SystemTraySubmenu {
    let worker_address = service.worker_address().unwrap();

    let mut submenu = SystemTrayMenu::new();
    // NOTE: Event handler for dynamic ID is defined in crate::invitations::tray_menu module,
    // and reached via crate::app::tray_menu::fallback_for_id
    submenu = submenu.add_item(CustomMenuItem::new(
        format!("invitation-create-for-{}", service.socket_addr),
        "Share...".to_string(),
    ));

    submenu = submenu.add_item(CustomMenuItem::new(
        "outlet-tcp-address".to_string(),
        format!("TCP Address: {}", service.socket_addr),
    ));

    let outlet_info = String::from_utf8(worker_address.last().unwrap().data().to_vec())
        .unwrap_or_else(|_| worker_address.to_string());

    SystemTraySubmenu::new(outlet_info, submenu)
}

/// Event listener for the "Create..." menu item
pub fn on_create(app: &AppHandle<Wry>) -> tauri::Result<()> {
    match app.get_window(SHARED_SERVICE_WINDOW_ID) {
        None => {
            let w = tauri::WindowBuilder::new(
                app,
                SHARED_SERVICE_WINDOW_ID,
                tauri::WindowUrl::App("service".into()),
            )
            .always_on_top(true)
            .visible(false)
            .title("Share a service")
            .max_inner_size(450.0, 350.0)
            .resizable(false)
            .minimizable(false)
            .build()?;
            // TODO: ideally we should use Position::TrayCenter, but it's broken on the latest alpha
            let _ = w.move_window(Position::TopRight);
            w.show()?;
        }
        Some(w) => w.set_focus()?,
    }
    Ok(())
}
