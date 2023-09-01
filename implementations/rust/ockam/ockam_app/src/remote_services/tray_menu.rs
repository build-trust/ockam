use std::net::SocketAddr;
use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTrayMenu, SystemTrayMenuItem,
    SystemTraySubmenu,
};
use tracing::{debug, trace};

use ockam_api::cloud::share::InvitationWithAccess;

use super::state::SyncState;
use crate::app::AppState;

pub const REMOTE_SERVICES_HEADER_MENU_ID: &str = "remote_services_header_menu";

pub(crate) async fn build_remote_services_section(
    app_handle: &AppHandle,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    let app_state: State<'_, AppState> = app_handle.state();
    if !app_state.is_enrolled().await.unwrap_or(false) {
        trace!("not enrolled, skipping remote_services menu");
        return tray_menu;
    };

    let state: State<'_, SyncState> = app_handle.state();
    let reader = state.read().await;
    let tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);

    let mut remote_services_submenu = SystemTrayMenu::new();
    remote_services_submenu = add_remote_services(remote_services_submenu, &reader.services.zip());

    tray_menu.add_submenu(SystemTraySubmenu::new(
        "Remote services",
        remote_services_submenu,
    ))
}

fn add_remote_services(
    submenu: SystemTrayMenu,
    service_invitations: &[(&InvitationWithAccess, Option<&SocketAddr>)],
) -> SystemTrayMenu {
    trace!(?service_invitations, "adding remote_services menu");
    if service_invitations.is_empty() {
        return submenu.add_item(
            CustomMenuItem::new(REMOTE_SERVICES_HEADER_MENU_ID, "No remote services").disabled(),
        );
    };
    let services_submenu = service_invitations
        .iter()
        .map(add_accepted_menu)
        .fold(SystemTrayMenu::new(), |menu, submenu| {
            menu.add_submenu(submenu)
        });
    submenu.add_submenu(SystemTraySubmenu::new("Remote services", services_submenu))
}

fn add_accepted_menu(
    service_invitation: &(&InvitationWithAccess, Option<&SocketAddr>),
) -> SystemTraySubmenu {
    let (invitation, inlet_socket_addr) = service_invitation;
    debug!(invitation_id = %invitation.invitation.id, ?inlet_socket_addr, "adding accepted invitation menu");
    let id = invitation.invitation.id.to_owned();
    let inlet_title = match inlet_socket_addr {
        Some(s) => format!("Listening at: {}", s),
        None => "Not connected".to_string(),
    };
    let service_submenu =
        SystemTrayMenu::new().add_item(CustomMenuItem::new(inlet_title, id.clone()).disabled());

    SystemTraySubmenu::new(
        format!("{}/service name", invitation.invitation.owner_email),
        service_submenu,
    )
}
