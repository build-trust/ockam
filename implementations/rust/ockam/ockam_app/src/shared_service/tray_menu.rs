use tauri::{AppHandle, CustomMenuItem, Manager, SystemTrayMenu, SystemTraySubmenu, Wry};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_runtime::menu::SystemTrayMenuItem;

use ockam_api::nodes::models::portal::OutletStatus;

use crate::app::AppState;

pub const SHARED_SERVICE_HEADER_MENU_ID: &str = "shared_service_header";
pub const SHARED_SERVICE_CREATE_MENU_ID: &str = "shared_service_create";
pub const SHARED_SERVICE_WINDOW_ID: &str = "shared_service_creation";

pub(crate) async fn build_shared_services_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    if !app_state.is_enrolled().await.unwrap_or(false) {
        return tray_menu;
    };

    let tm = tray_menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new(SHARED_SERVICE_HEADER_MENU_ID, "Shared").disabled())
        .add_item(CustomMenuItem::new(
            SHARED_SERVICE_CREATE_MENU_ID,
            "Create...",
        ));
    app_state
        .tcp_outlet_list()
        .await
        .iter()
        .map(shared_service_submenu)
        .fold(tm, |menu, submenu| menu.add_submenu(submenu))
}

fn shared_service_submenu(outlet: &OutletStatus) -> SystemTraySubmenu {
    let worker_address = outlet.worker_address().unwrap();

    let mut submenu = SystemTrayMenu::new();
    // NOTE: Event handler for dynamic ID is defined in crate::invitations::tray_menu module,
    // and reached via crate::app::tray_menu::fallback_for_id
    submenu = submenu.add_item(CustomMenuItem::new(
        format!("invitation-create-for-{}", outlet.socket_addr),
        "Share".to_string(),
    ));

    submenu = submenu.add_item(
        CustomMenuItem::new(
            "outlet-tcp-address".to_string(),
            format!("TCP Address: {}", outlet.socket_addr),
        )
        .disabled(),
    );

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
