use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTrayMenu, SystemTrayMenuItem,
    SystemTraySubmenu,
};
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
    sent.iter().map(sent_invitation_menu).fold(
        tray_menu.add_item(
            CustomMenuItem::new(INVITATIONS_PENDING_HEADER_MENU_ID, header_text).disabled(),
        ),
        |menu, submenu| menu.add_submenu(submenu),
    )
}

fn sent_invitation_menu(invitation: &SentInvitation) -> SystemTraySubmenu {
    let id = invitation.id.to_owned();
    let submenu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(id.clone(), id.clone()).disabled())
        .add_item(CustomMenuItem::new(id.clone(), invitation.recipient_email.to_owned()).disabled())
        .add_item(
            CustomMenuItem::new(
                format!("invitation-sent-cancel-{}", invitation.id),
                "Cancel",
            )
            .disabled(),
        );
    SystemTraySubmenu::new(id, submenu)
}

fn add_received_menu(tray_menu: SystemTrayMenu, received: &[ReceivedInvitation]) -> SystemTrayMenu {
    let header_text = if received.is_empty() {
        "No Received Invitations"
    } else {
        "Received Invitations"
    };
    received.iter().map(received_invite_menu).fold(
        tray_menu.add_item(
            CustomMenuItem::new(INVITATIONS_RECEIVED_HEADER_MENU_ID, header_text).disabled(),
        ),
        |menu, submenu| menu.add_submenu(submenu),
    )
}

fn received_invite_menu(invitation: &ReceivedInvitation) -> SystemTraySubmenu {
    let id = invitation.id.to_owned();
    let submenu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(id.clone(), id.clone()).disabled())
        .add_item(
            CustomMenuItem::new(id.clone(), format!("Sent by: {}", invitation.owner_email))
                .disabled(),
        )
        .add_item(
            CustomMenuItem::new(
                id.clone(),
                format!("Grants role: {}", invitation.grant_role),
            )
            .disabled(),
        )
        .add_item(
            CustomMenuItem::new(
                id.clone(),
                format!("Target: {} {}", invitation.scope, invitation.target_id),
            )
            .disabled(),
        )
        .add_item(CustomMenuItem::new(
            format!("invitation-received-accept-{}", invitation.id),
            "Accept",
        ))
        .add_item(
            CustomMenuItem::new(
                format!("invitation-received-decline-{}", invitation.id),
                "Decline",
            )
            .disabled(),
        );
    SystemTraySubmenu::new(id, submenu)
}

fn add_accepted_menu(
    tray_menu: SystemTrayMenu,
    accepted: &[InvitationWithAccess],
) -> SystemTrayMenu {
    let header_text = if accepted.is_empty() {
        "No Accepted Invitations"
    } else {
        "Accepted Invitations"
    };
    accepted.iter().map(accepted_invite_menu).fold(
        tray_menu.add_item(
            CustomMenuItem::new(INVITATIONS_ACCEPTED_HEADER_MENU_ID, header_text).disabled(),
        ),
        |menu, submenu| menu.add_submenu(submenu),
    )
}

fn accepted_invite_menu(invitation: &InvitationWithAccess) -> SystemTraySubmenu {
    let id = invitation.invitation.id.to_owned();
    let submenu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(id.clone(), id.clone()).disabled())
        .add_item(
            CustomMenuItem::new(
                id.clone(),
                format!("Sent by: {}", invitation.invitation.owner_email),
            )
            .disabled(),
        )
        .add_item(
            CustomMenuItem::new(
                id.clone(),
                format!("Grants role: {}", invitation.invitation.grant_role),
            )
            .disabled(),
        )
        .add_item(
            CustomMenuItem::new(
                id.clone(),
                format!(
                    "Target: {} {}",
                    invitation.invitation.scope, invitation.invitation.target_id
                ),
            )
            .disabled(),
        )
        .add_item(
            CustomMenuItem::new(
                format!("invitation-accepted-connect-{}", invitation.invitation.id),
                "Connect",
            )
            .disabled(),
        )
        .add_item(
            CustomMenuItem::new(
                format!("invitation-accepted-leave-{}", invitation.invitation.id),
                "Leave",
            )
            .disabled(),
        );

    SystemTraySubmenu::new(id, submenu)
}
