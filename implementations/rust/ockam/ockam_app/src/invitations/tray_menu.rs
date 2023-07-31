use tauri::{CustomMenuItem, SystemTrayMenu};
use tauri_runtime::menu::SystemTrayMenuItem;

use crate::app::AppState;

pub const INVITATIONS_SENT_HEADER_MENU_ID: &str = "sent_invitations_header";
pub const INVITATIONS_RECEIVED_HEADER_MENU_ID: &str = "received_invitations_header";

pub(crate) async fn build_invitations_section(
    app_state: &AppState,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    if !app_state.is_enrolled().await {
        return tray_menu;
    };

    tray_menu
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(
            CustomMenuItem::new(INVITATIONS_SENT_HEADER_MENU_ID, "Sent Invitations").disabled(),
        )
        .add_item(
            CustomMenuItem::new(INVITATIONS_RECEIVED_HEADER_MENU_ID, "Received Invitations")
                .disabled(),
        )
}
