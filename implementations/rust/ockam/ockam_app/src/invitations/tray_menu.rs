use tauri::menu::{MenuBuilder, MenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{debug, trace, warn};

use ockam_api::cloud::share::{InvitationWithAccess, ReceivedInvitation, SentInvitation};

use super::state::SyncState;
use crate::app::AppState;

pub const INVITATIONS_PENDING_HEADER_MENU_ID: &str = "sent_invitations_header";
pub const INVITATIONS_RECEIVED_HEADER_MENU_ID: &str = "received_invitations_header";
pub const INVITATIONS_ACCEPTED_HEADER_MENU_ID: &str = "accepted_invitations_header";
pub const INVITATIONS_MANAGE_MENU_ID: &str = "invitations_manage";

pub(crate) async fn build_invitations_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<'_, AppState> = app_handle.state();
    if !app_state.is_enrolled().await {
        return builder;
    };

    let state: State<'_, SyncState> = app_handle.state();
    let reader = state.read().await;
    debug!(sent = ?reader.sent, received = ?reader.received);

    builder = builder.separator();
    builder = builder.item(&add_pending_menu(app_handle, &reader.sent));
    builder = builder.item(&add_received_menu(app_handle, &reader.received));
    builder = builder.item(&add_accepted_menu(app_handle, &reader.accepted));

    builder.item(&MenuItem::with_id(
        app_handle,
        INVITATIONS_MANAGE_MENU_ID,
        "Manage Invitations...",
        false,
        None,
    ))
}

fn add_pending_menu<R: Runtime>(app_handle: &AppHandle<R>, sent: &[SentInvitation]) -> Submenu<R> {
    let header_text = if sent.is_empty() {
        "No Pending Invitations"
    } else {
        "Pending Invitations"
    };

    let mut submenu_builder =
        SubmenuBuilder::with_id(app_handle, INVITATIONS_PENDING_HEADER_MENU_ID, header_text);

    submenu_builder = sent
        .iter()
        .map(|invitation| sent_invitation_menu(app_handle, invitation))
        .fold(submenu_builder, |submenu_builder, submenu| {
            submenu_builder.item(&submenu)
        });

    submenu_builder
        .build()
        .expect("cannot build pending submenu")
}

fn sent_invitation_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    invitation: &SentInvitation,
) -> Submenu<R> {
    let id = invitation.id.to_owned();

    SubmenuBuilder::with_id(app_handle, id.clone(), &id)
        .items(&[
            &MenuItem::with_id(app_handle, id.clone(), id.clone(), false, None),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                invitation.recipient_email.to_owned(),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                format!("invitation-sent-cancel-{}", invitation.id),
                "Cancel".to_owned(),
                false,
                None,
            ),
        ])
        .build()
        .expect("cannot build single invitation submenu")
}

fn add_received_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    received: &[ReceivedInvitation],
) -> Submenu<R> {
    let header_text = if received.is_empty() {
        "No Received Invitations"
    } else {
        "Received Invitations"
    };

    let mut submenu_builder =
        SubmenuBuilder::with_id(app_handle, INVITATIONS_RECEIVED_HEADER_MENU_ID, header_text);

    submenu_builder = received
        .iter()
        .map(|invitation| received_invite_menu(app_handle, invitation))
        .fold(submenu_builder, |submenu_builder, submenu| {
            submenu_builder.item(&submenu)
        });

    submenu_builder
        .build()
        .expect("cannot build received submenu")
}

fn received_invite_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    invitation: &ReceivedInvitation,
) -> Submenu<R> {
    let id = invitation.id.to_owned();

    SubmenuBuilder::with_id(app_handle, id.clone(), &id)
        .items(&[
            &MenuItem::with_id(app_handle, id.clone(), id.clone(), false, None),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                format!("Sent by: {}", invitation.owner_email),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                format!("Grants role: {}", invitation.grant_role),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                format!("Target: {} {}", invitation.scope, invitation.target_id),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                format!("invitation-received-accept-{}", invitation.id),
                "Accept".to_owned(),
                true,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                format!("invitation-received-decline-{}", invitation.id),
                "Decline".to_owned(),
                false,
                None,
            ),
        ])
        .build()
        .expect("cannot build received invitation submenu")
}

fn add_accepted_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    accepted: &[InvitationWithAccess],
) -> Submenu<R> {
    let header_text = if accepted.is_empty() {
        "No Accepted Invitations"
    } else {
        "Accepted Invitations"
    };

    let mut submenu_builder =
        SubmenuBuilder::with_id(app_handle, INVITATIONS_ACCEPTED_HEADER_MENU_ID, header_text);

    submenu_builder = accepted
        .iter()
        .map(|invitation| accepted_invite_menu(app_handle, invitation))
        .fold(submenu_builder, |menu, submenu| menu.item(&submenu));

    submenu_builder
        .build()
        .expect("cannot build accepted submenu")
}

fn accepted_invite_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    invitation: &InvitationWithAccess,
) -> Submenu<R> {
    let id = invitation.invitation.id.to_owned();
    SubmenuBuilder::with_id(app_handle, id.clone(), &id)
        .items(&[
            &MenuItem::with_id(app_handle, id.clone(), id.clone(), false, None),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                format!("Sent by: {}", invitation.invitation.owner_email),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                format!("Grants role: {}", invitation.invitation.grant_role),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                id.clone(),
                format!(
                    "Target: {} {}",
                    invitation.invitation.scope, invitation.invitation.target_id
                ),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                format!("invitation-accepted-connect-{}", invitation.invitation.id),
                "Connect".to_owned(),
                false,
                None,
            ),
            &MenuItem::with_id(
                app_handle,
                format!("invitation-accepted-leave-{}", invitation.invitation.id),
                "Leave".to_owned(),
                false,
                None,
            ),
        ])
        .build()
        .expect("cannot build accepted invitation submenu")
}

pub(crate) fn dispatch_click_event<R: Runtime>(app: &AppHandle<R>, id: &str) -> tauri::Result<()> {
    let segments = id
        .splitn(4, '-')
        .skip_while(|segment| segment == &"invitation")
        .collect::<Vec<&str>>();
    match segments.as_slice() {
        ["create", "for", outlet_tcp_addr] => on_create(app, outlet_tcp_addr),
        ["accepted", "connect", id] => on_connect(app, id),
        ["received", "accept", id] => on_accept(app, id),
        ["received", "decline", id] => on_decline(app, id),
        ["sent", "cancel", id] => on_cancel(app, id),
        other => {
            warn!(?other, "unexpected menu ID");
            Ok(())
        }
    }
}

fn on_create<R: Runtime>(_app: &AppHandle<R>, outlet_tcp_addr: &str) -> tauri::Result<()> {
    trace!(?outlet_tcp_addr, "create service invitation");
    todo!("open window to ask the user for the recipient email address");
}

fn on_accept<R: Runtime>(app: &AppHandle<R>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "accepting invite via spawn");

    let app_handle = app.clone();
    let invite_id = invite_id.to_string();
    tauri::async_runtime::spawn(async move {
        super::commands::accept_invitation(invite_id, app_handle).await
    });

    Ok(())
}

fn on_cancel<R: Runtime>(_app: &AppHandle<R>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "canceling invite via spawn");
    todo!()
}

fn on_connect<R: Runtime>(_app: &AppHandle<R>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "connecting to service via spawn");
    todo!()
}

fn on_decline<R: Runtime>(_app: &AppHandle<R>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "declining invite via spawn");
    todo!()
}
