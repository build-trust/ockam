use crate::app::AppState;
use crate::tcp::outlet::tcp_outlet_create;
use tauri::{AppHandle, CustomMenuItem, SystemTrayMenu, Wry};

pub const TCP_OUTLET_HEADER_MENU_ID: &str = "tcp_outlet_header";
pub const TCP_OUTLET_CREATE_MENU_ID: &str = "tcp_outlet_create";

pub(crate) fn build_outlets_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    let mut tm = tray_menu
        .add_item(
            CustomMenuItem::new(TCP_OUTLET_HEADER_MENU_ID, "TCP Outlets")
                .disabled()
                .into(),
        )
        .add_item(CustomMenuItem::new(TCP_OUTLET_CREATE_MENU_ID, "Create...").into());
    for outlet in app_state.tcp_outlet_list() {
        let outlet_info = format!(
            "{} to {}",
            outlet.worker_address().unwrap(),
            outlet.tcp_addr
        );
        let item = CustomMenuItem::new(outlet_info.clone(), outlet_info).into();
        tm = tm.add_item(item);
    }
    tm
}

/// Event listener for the "Create..." menu item
pub fn on_create(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { tcp_outlet_create(&app).await });
    Ok(())
}
