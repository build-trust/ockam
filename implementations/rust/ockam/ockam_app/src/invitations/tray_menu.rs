use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use std::net::SocketAddr;
use tauri::menu::{MenuBuilder, MenuEvent, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_positioner::{Position, WindowExt};
use tracing::{debug, trace, warn};

use ockam_api::cloud::share::{InvitationWithAccess, ReceivedInvitation, SentInvitation};

use super::state::SyncState;
use crate::app::AppState;
use crate::invitations::state::AcceptedInvitations;

pub const INVITATIONS_PENDING_HEADER_MENU_ID: &str = "sent_invitations_header";
pub const INVITATIONS_RECEIVED_HEADER_MENU_ID: &str = "received_invitations_header";
pub const INVITATIONS_ACCEPTED_HEADER_MENU_ID: &str = "accepted_invitations_header";
pub const INVITATIONS_WINDOW_ID: &str = "invitations_creation";

// https://url.spec.whatwg.org/#path-percent-encode-set
const PATH_ENCODING_SET: AsciiSet = CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

pub(crate) async fn build_invitations_section<'a, R: Runtime, M: Manager<R>>(
    app_handle: &AppHandle<R>,
    mut builder: MenuBuilder<'a, R, M>,
) -> MenuBuilder<'a, R, M> {
    let app_state: State<'_, AppState> = app_handle.state();
    if !app_state.is_enrolled().await.unwrap_or(false) {
        trace!("not enrolled, skipping invitations menu");
        return builder;
    };

    let state: State<'_, SyncState> = app_handle.state();
    let reader = state.read().await;
    builder = builder.item(&add_received_menu(app_handle, &reader.received));

    builder.item(&add_manage_submenu(
        app_handle,
        &reader.accepted,
        &reader.sent,
    ))
}

fn add_manage_submenu<R: Runtime>(
    app_handle: &AppHandle<R>,
    accepted: &AcceptedInvitations,
    sent: &[SentInvitation],
) -> Submenu<R> {
    SubmenuBuilder::new(app_handle, "Manage Invitations...")
        .items(&[
            &add_pending_menu(app_handle, sent),
            &add_accepted_menu(app_handle, accepted),
        ])
        .build()
        .expect("manage invitation menu build failed")
}

fn add_pending_menu<R: Runtime>(app_handle: &AppHandle<R>, sent: &[SentInvitation]) -> Submenu<R> {
    trace!(?sent, "adding pending invitations menu");
    let header_text = if sent.is_empty() {
        "No Pending Invitations"
    } else {
        "Pending Invitations"
    };

    let mut submenu_builder =
        SubmenuBuilder::with_id(app_handle, INVITATIONS_PENDING_HEADER_MENU_ID, header_text);

    submenu_builder = sent
        .iter()
        .map(|invitation| pending_invitation_menu(app_handle, invitation))
        .fold(submenu_builder, |submenu_builder, submenu| {
            submenu_builder.item(&submenu)
        });

    submenu_builder
        .build()
        .expect("cannot build pending submenu")
}

fn pending_invitation_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    invitation: &SentInvitation,
) -> Submenu<R> {
    let id = invitation.id.to_owned();

    SubmenuBuilder::with_id(app_handle, id.clone(), &id)
        .items(&[
            &MenuItemBuilder::with_id(id.clone(), id.clone())
                .enabled(true)
                .build(app_handle),
            &MenuItemBuilder::with_id(id.clone(), &invitation.recipient_email)
                .enabled(false)
                .build(app_handle),
            &MenuItemBuilder::with_id(
                format!("invitation-sent-cancel-{}", invitation.id),
                "Cancel",
            )
            .enabled(false)
            .build(app_handle),
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
            &MenuItemBuilder::with_id(id.clone(), id.clone())
                .enabled(false)
                .build(app_handle),
            &MenuItemBuilder::with_id(id.clone(), format!("Sent by: {}", invitation.owner_email))
                .enabled(false)
                .build(app_handle),
            &MenuItemBuilder::with_id(
                id.clone(),
                format!("Grants role: {}", invitation.grant_role),
            )
            .enabled(false)
            .build(app_handle),
            &MenuItemBuilder::with_id(
                id.clone(),
                format!("Target: {} {}", invitation.scope, invitation.target_id),
            )
            .enabled(false)
            .build(app_handle),
            &MenuItemBuilder::with_id(
                format!("invitation-received-accept-{}", invitation.id),
                "Accept",
            )
            .build(app_handle),
            &MenuItemBuilder::with_id(
                format!("invitation-received-decline-{}", invitation.id),
                "Decline",
            )
            .enabled(false)
            .build(app_handle),
        ])
        .build()
        .expect("cannot build received invitation submenu")
}

fn add_accepted_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    accepted: &AcceptedInvitations,
) -> Submenu<R> {
    let header_text = if accepted.invitations.is_empty() {
        "No Accepted Invitations"
    } else {
        "Accepted Invitations"
    };

    let mut submenu_builder =
        SubmenuBuilder::with_id(app_handle, INVITATIONS_ACCEPTED_HEADER_MENU_ID, header_text);

    submenu_builder = accepted
        .zip()
        .iter()
        .map(|(invitation, inlet_socket_addr)| {
            accepted_invite_menu(app_handle, invitation, inlet_socket_addr)
        })
        .fold(submenu_builder, |menu, submenu| menu.item(&submenu));

    submenu_builder
        .build()
        .expect("cannot build accepted submenu")
}

fn accepted_invite_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    invitation: &InvitationWithAccess,
    inlet_socket_addr: &Option<&SocketAddr>,
) -> Submenu<R> {
    let id = invitation.invitation.id.to_owned();
    let inlet_title = match inlet_socket_addr {
        Some(s) => format!("Listening at: {}", s),
        None => "Not connected".to_string(),
    };

    SubmenuBuilder::with_id(app_handle, id.clone(), &id)
        .items(&[
            &MenuItemBuilder::with_id(id.clone(), id.clone())
                .enabled(false)
                .build(app_handle),
            &MenuItemBuilder::with_id(
                id.clone(),
                format!("Sent by: {}", invitation.invitation.owner_email),
            )
            .enabled(false)
            .build(app_handle),
            &MenuItemBuilder::with_id(
                id.clone(),
                format!("Grants role: {}", invitation.invitation.grant_role),
            )
            .enabled(false)
            .build(app_handle),
            &MenuItemBuilder::with_id(
                id.clone(),
                format!(
                    "Target: {} {}",
                    invitation.invitation.scope, invitation.invitation.target_id
                ),
            )
            .enabled(false)
            .build(app_handle),
            &MenuItemBuilder::with_id(
                format!("invitation-accepted-connect-{}", invitation.invitation.id),
                inlet_title,
            )
            .enabled(false)
            .build(app_handle),
            &MenuItemBuilder::with_id(
                format!("invitation-accepted-leave-{}", invitation.invitation.id),
                "Leave",
            )
            .enabled(false)
            .build(app_handle),
        ])
        .build()
        .expect("cannot build accepted invitation submenu")
}

pub fn process_tray_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &MenuEvent,
) -> tauri::Result<()> {
    match event.id.as_ref() {
        id => {
            if id.starts_with("invitation-") {
                dispatch_click_event(app, id)
            } else {
                Ok(())
            }
        }
    }
}

fn dispatch_click_event<R: Runtime>(app: &AppHandle<R>, id: &str) -> tauri::Result<()> {
    let segments = id
        .splitn(4, '-')
        .skip_while(|segment| segment == &"invitation")
        .collect::<Vec<&str>>();
    match segments.as_slice() {
        ["accepted", "connect", id] => on_connect(app, id),
        ["create", "for", outlet_socket_addr] => on_create(app, outlet_socket_addr),
        ["received", "accept", id] => on_accept(app, id),
        ["received", "decline", id] => on_decline(app, id),
        ["sent", "cancel", id] => on_cancel(app, id),
        other => {
            warn!(?other, "unexpected menu ID");
            Ok(())
        }
    }
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

fn on_create<R: Runtime>(app: &AppHandle<R>, outlet_socket_addr: &str) -> tauri::Result<()> {
    debug!(?outlet_socket_addr, "creating invite via menu");

    match app.get_window(INVITATIONS_WINDOW_ID) {
        None => {
            let url_path = percent_encode(
                format!("invite/{outlet_socket_addr}").as_bytes(),
                &PATH_ENCODING_SET,
            )
            .to_string();
            let w = tauri::WindowBuilder::new(
                app,
                INVITATIONS_WINDOW_ID,
                tauri::WindowUrl::App(url_path.into()),
            )
            .always_on_top(true)
            .visible(false)
            .title("Invite To Share")
            .max_inner_size(640.0, 480.0)
            .resizable(true)
            .minimizable(false)
            .build()?;
            // TODO: ideally we should use Position::TrayCenter, but it's broken on the latest alpha
            let _ = w.move_window(Position::TopRight);
            w.show()?;

            #[cfg(debug_assertions)]
            {
                let app_state: State<AppState> = app.state();
                if app_state.browser_dev_tools() {
                    w.open_devtools();
                }
            }
        }
        Some(w) => w.set_focus()?,
    }
    Ok(())
}

fn on_connect<R: Runtime>(_app: &AppHandle<R>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "connecting to service via spawn");
    todo!()
}

fn on_decline<R: Runtime>(_app: &AppHandle<R>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "declining invite via spawn");
    todo!()
}
