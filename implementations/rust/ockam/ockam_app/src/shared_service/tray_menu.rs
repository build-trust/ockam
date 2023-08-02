use tauri::{AppHandle, CustomMenuItem, Manager, SystemTrayMenu, Wry};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_runtime::menu::SystemTrayMenuItem;

use crate::app::AppState;

pub const SHARED_SERVICE_HEADER_MENU_ID: &str = "shared_service_header";
pub const SHARED_SERVICE_CREATE_MENU_ID: &str = "shared_service_create";
pub const SHARED_SERVICE_WINDOW_ID: &str = "shared_service_creation";

pub(crate) async fn build_shared_services_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    if !app_state.is_enrolled().await {
        return tray_menu;
    };

    let mut tm = tray_menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new(SHARED_SERVICE_HEADER_MENU_ID, "Shared").disabled())
        .add_item(CustomMenuItem::new(
            SHARED_SERVICE_CREATE_MENU_ID,
            "Create...",
        ));
    for outlet in app_state.tcp_outlet_list().await {
        let outlet_info = format!(
            "{} to {}",
            outlet.worker_address().unwrap(),
            outlet.tcp_addr
        );
        let item = CustomMenuItem::new(outlet_info.clone(), outlet_info);
        tm = tm.add_item(item);
    }
    tm
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
            .max_inner_size(400.0, 400.0)
            .resizable(false)
            .minimizable(false)
            .build()?;
            // TODO: ideally we should use Position::TrayCenter, but it's broken on the latest alpha
            let _ = w.move_window(Position::TopRight);
            w.show()?;
        }
        Some(w) => w.show()?,
    }
    Ok(())
}
