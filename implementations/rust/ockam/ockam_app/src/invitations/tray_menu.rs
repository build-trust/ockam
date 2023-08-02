use tauri::{AppHandle, CustomMenuItem, Manager, State, SystemTrayMenu};
use tauri_runtime::menu::SystemTrayMenuItem;
use tracing::debug;

use ockam_api::cloud::share::{InvitationWithAccess, ReceivedInvitation, SentInvitation};

use crate::app::AppState;

use super::state::SyncState;

pub const INVITATIONS_PENDING_HEADER_MENU_ID: &str = "sent_invitations_header";
pub const INVITATIONS_RECEIVED_HEADER_MENU_ID: &str = "received_invitations_header";
pub const INVITATIONS_ACCEPTED_HEADER_MENU_ID: &str = "accepted_invitations_header";
pub const INVITATIONS_MANAGE_MENU_ID: &str = "invitations_manage";

pub(crate) async fn build_invitations_section(
    app_handle: &AppHandle,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    let app_state: State<'_, AppState> = app_handle.state();
    if !app_state.is_enrolled().await {
        return tray_menu;
    };

    let state: State<'_, SyncState> = app_handle.state();
    let reader = state.read().await;
    debug!(sent = ?reader.sent, received = ?reader.received);

    let mut tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);
    tray_menu = add_pending_menu(tray_menu, &reader.sent);
    tray_menu = add_received_menu(tray_menu, &reader.received);
    tray_menu = add_accepted_menu(tray_menu, &reader.accepted);
    tray_menu.add_item(
        CustomMenuItem::new(INVITATIONS_MANAGE_MENU_ID, "Manage Invitations...").disabled(),
    )
}

fn add_pending_menu(tray_menu: SystemTrayMenu, sent: &[SentInvitation]) -> SystemTrayMenu {
    let header_text = if sent.is_empty() {
        "No Pending Invitations"
    } else {
        "Pending Invitations"
    };
    sent.iter().map(sent_menu_item).fold(
        tray_menu.add_item(
            CustomMenuItem::new(INVITATIONS_PENDING_HEADER_MENU_ID, header_text).disabled(),
        ),
        |menu, entry| menu.add_item(entry),
    )
}

fn sent_menu_item(invitation: &SentInvitation) -> CustomMenuItem {
    CustomMenuItem::new(invitation.id.to_owned(), invitation.id.to_owned())
}

fn add_received_menu(tray_menu: SystemTrayMenu, received: &[ReceivedInvitation]) -> SystemTrayMenu {
    let header_text = if received.is_empty() {
        "No Received Invitations"
    } else {
        "Received Invitations"
    };
    received.iter().map(received_menu_item).fold(
        tray_menu.add_item(
            CustomMenuItem::new(INVITATIONS_RECEIVED_HEADER_MENU_ID, header_text).disabled(),
        ),
        |menu, entry| menu.add_item(entry),
    )
}

fn received_menu_item(invitation: &ReceivedInvitation) -> CustomMenuItem {
    CustomMenuItem::new(invitation.id.to_owned(), invitation.id.to_owned())
}

fn add_accepted_menu(
    tray_menu: SystemTrayMenu,
    accepted: &[InvitationWithAccess],
) -> SystemTrayMenu {
    let header_text = if accepted.is_empty() {
        "No accepted Invitations"
    } else {
        "accepted Invitations"
    };
    accepted.iter().map(accepted_menu_item).fold(
        tray_menu.add_item(
            CustomMenuItem::new(INVITATIONS_ACCEPTED_HEADER_MENU_ID, header_text).disabled(),
        ),
        |menu, entry| menu.add_item(entry),
    )
}

fn accepted_menu_item(invitation: &InvitationWithAccess) -> CustomMenuItem {
    CustomMenuItem::new(
        invitation.invitation.id.to_owned(),
        invitation.invitation.id.to_owned(),
    )
}
