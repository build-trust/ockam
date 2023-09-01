use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTrayMenu, SystemTrayMenuItem,
    SystemTraySubmenu, Wry,
};
use tauri_plugin_positioner::{Position, WindowExt};
use tracing::{debug, trace, warn};

use crate::app::AppState;
use crate::invitations::state::SyncState;
use ockam_api::cloud::share::{ReceivedInvitation, SentInvitation};

pub const INVITATIONS_PENDING_HEADER_MENU_ID: &str = "sent_invitations_header";
pub const INVITATIONS_RECEIVED_HEADER_MENU_ID: &str = "received_invitations_header";
pub const INVITATIONS_WINDOW_ID: &str = "invitations_creation";

pub(crate) async fn build_invitations_section(
    app_handle: &AppHandle,
    tray_menu: SystemTrayMenu,
) -> SystemTrayMenu {
    let app_state: State<'_, AppState> = app_handle.state();
    if !app_state.is_enrolled().await.unwrap_or(false) {
        trace!("not enrolled, skipping invitations menu");
        return tray_menu;
    };

    let state: State<'_, SyncState> = app_handle.state();
    let reader = state.read().await;
    let tray_menu = tray_menu.add_native_item(SystemTrayMenuItem::Separator);

    let mut invitations_submenu = SystemTrayMenu::new();
    invitations_submenu = add_sent_menu(invitations_submenu, &reader.sent);
    invitations_submenu = add_received_menu(invitations_submenu, &reader.received);

    tray_menu.add_submenu(SystemTraySubmenu::new("Invitations", invitations_submenu))
}

fn add_sent_menu(submenu: SystemTrayMenu, sent: &[SentInvitation]) -> SystemTrayMenu {
    trace!(?sent, "adding sent invitations menu");
    if sent.is_empty() {
        return submenu.add_item(
            CustomMenuItem::new(INVITATIONS_PENDING_HEADER_MENU_ID, "No Pending Invitations")
                .disabled(),
        );
    };
    let sent_submenu = sent
        .iter()
        .map(sent_invitation_menu)
        .fold(SystemTrayMenu::new(), |menu, submenu| {
            menu.add_submenu(submenu)
        });
    submenu.add_submenu(SystemTraySubmenu::new("Sent invitations", sent_submenu))
}

fn sent_invitation_menu(invitation: &SentInvitation) -> SystemTraySubmenu {
    let id = invitation.id.to_owned();
    let submenu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(
            id.clone(),
            format!("Invitation id: {}", id.clone()),
        ))
        .add_item(
            CustomMenuItem::new(
                format!("invitation-sent-cancel-{}", invitation.id),
                "Cancel",
            )
            .disabled(),
        );
    SystemTraySubmenu::new(
        format!("service name/{}", invitation.recipient_email),
        submenu,
    )
}

fn add_received_menu(submenu: SystemTrayMenu, received: &[ReceivedInvitation]) -> SystemTrayMenu {
    trace!(?received, "adding received invitations menu");
    if received.is_empty() {
        return submenu.add_item(
            CustomMenuItem::new(
                INVITATIONS_RECEIVED_HEADER_MENU_ID,
                "No Received Invitations",
            )
            .disabled(),
        );
    };
    let received_submenu = received
        .iter()
        .map(received_invitation_menu)
        .fold(SystemTrayMenu::new(), |menu, submenu| {
            menu.add_submenu(submenu)
        });
    submenu.add_submenu(SystemTraySubmenu::new(
        "Received invitations",
        received_submenu,
    ))
}

fn received_invitation_menu(invitation: &ReceivedInvitation) -> SystemTraySubmenu {
    let id = invitation.id.to_owned();
    let submenu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(
            id.clone(),
            format!("Invitation id: {}", id.clone()),
        ))
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
    SystemTraySubmenu::new(format!("{}/service name", invitation.owner_email), submenu)
}

pub(crate) fn dispatch_click_event(app: &AppHandle<Wry>, id: &str) -> tauri::Result<()> {
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

fn on_accept(app: &AppHandle<Wry>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "accepting invite via spawn");

    let app_handle = app.clone();
    let invite_id = invite_id.to_string();
    tauri::async_runtime::spawn(async move {
        super::commands::accept_invitation(invite_id, app_handle).await
    });

    Ok(())
}

fn on_cancel(_app: &AppHandle<Wry>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "canceling invite via spawn");
    todo!()
}

fn on_create(app: &AppHandle<Wry>, outlet_socket_addr: &str) -> tauri::Result<()> {
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
        }
        Some(w) => w.set_focus()?,
    }
    Ok(())
}

fn on_connect(_app: &AppHandle<Wry>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "connecting to service via spawn");
    todo!()
}

fn on_decline(_app: &AppHandle<Wry>, invite_id: &str) -> tauri::Result<()> {
    trace!(?invite_id, "declining invite via spawn");
    todo!()
}

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
