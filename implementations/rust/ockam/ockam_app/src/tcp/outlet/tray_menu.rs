use crate::app::AppState;
use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, Wry};
use tauri_runtime::menu::SystemTrayMenuItem;

pub const SERVICE_HEADER_MENU_ID: &str = "service_outlet_header";
pub const SERVICE_CREATE_MENU_ID: &str = "service_outlet_create";
pub const SERVICE_WINDOW_ID: &str = "service_creation";

pub(crate) async fn build_outlets_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    if !app_state.is_enrolled() {
        return tray_menu;
    };

    let mut tm = tray_menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new(SERVICE_HEADER_MENU_ID, "Shared services").disabled())
        .add_item(CustomMenuItem::new(SERVICE_CREATE_MENU_ID, "Create..."));
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
    tauri::WindowBuilder::new(
        app,
        SERVICE_WINDOW_ID,
        tauri::WindowUrl::App("service".into()),
    )
    .build()?;
    Ok(())
}
