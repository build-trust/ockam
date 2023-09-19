use arboard::Clipboard;
use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use std::collections::HashMap;
use std::net::SocketAddr;
use tauri::async_runtime::spawn;
use tauri::menu::{
    IconMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder, NativeIcon, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Icon, Manager, Runtime, State};
use tauri_plugin_positioner::{Position, WindowExt};
use tracing::{debug, error, trace, warn};

use ockam_api::cloud::share::{ReceivedInvitation, SentInvitation, ServiceAccessDetails};

use super::state::SyncInvitationsState;
use crate::app::AppState;
use crate::invitations::state::AcceptedInvitations;

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

    let state: State<'_, SyncInvitationsState> = app_handle.state();
    let reader = state.read().await;

    let mut menu_items = vec![];
    if !reader.received.is_empty() {
        menu_items.push(add_received_menu(app_handle, &reader.received));
    }
    if !reader.accepted.invitations.is_empty() {
        add_accepted_menus(app_handle, &reader.accepted)
            .into_iter()
            .for_each(|s| menu_items.push(s));
    }

    builder = builder.item(
        &MenuItemBuilder::new("Shared services with you")
            .enabled(false)
            .build(app_handle),
    );

    builder = if menu_items.is_empty() {
        builder.item(
            &MenuItemBuilder::new("When they share a service with you they will appear here")
                .enabled(false)
                .build(app_handle),
        )
    } else {
        menu_items
            .into_iter()
            .fold(builder, |builder, submenu| builder.item(&submenu))
    };

    builder.separator()
}

pub(crate) fn pending_invitation_menu<R: Runtime>(
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
        ])
        .build()
        .expect("cannot build single invitation submenu")
}

fn add_received_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    received: &[ReceivedInvitation],
) -> Submenu<R> {
    debug!(
        count = received.len(),
        "Building menu for received invitations"
    );

    let header_text = format!(
        "{} pending invite{}",
        received.len(),
        if received.len() == 1 { "" } else { "s" }
    );

    let mut submenu_builder = SubmenuBuilder::new(app_handle, header_text);
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
    SubmenuBuilder::new(app_handle, &invitation.owner_email)
        .items(&[
            &MenuItemBuilder::new(&invitation.target_id)
                .enabled(false)
                .build(app_handle),
            &IconMenuItemBuilder::with_id(
                format!("invitation-received-accept-{}", invitation.id),
                "Accept invite",
            )
            .icon(Icon::Raw(
                include_bytes!("../../icons/check-lg.png").to_vec(),
            ))
            .build(app_handle),
        ])
        .build()
        .expect("cannot build received invitation submenu")
}

fn add_accepted_menus<R: Runtime>(
    app_handle: &AppHandle<R>,
    accepted: &AcceptedInvitations,
) -> Vec<Submenu<R>> {
    debug!(
        count = accepted.invitations.len(),
        "Building menu for accepted invitations"
    );

    // Group invitations by owner email and get the attached inlet for each invitation
    let mut invitations_by_owner = HashMap::new();
    for invitation in &accepted.invitations {
        if let Some(access_details) = &invitation.service_access_details {
            let invitations = invitations_by_owner
                .entry(invitation.invitation.owner_email.clone())
                .or_insert_with(Vec::new);
            invitations.push((
                &invitation.invitation.id,
                access_details,
                accepted.inlets.get(&invitation.invitation.id),
            ));
        }
    }

    // Build a submenu for each owner
    let mut submenus = Vec::new();
    for (owner_email, invitations) in invitations_by_owner {
        let mut submenu_builder = SubmenuBuilder::new(app_handle, owner_email);
        submenu_builder = invitations
            .into_iter()
            .map(|(invitation_id, access_details, inlet_socket_addr)| {
                accepted_invite_menu(app_handle, invitation_id, access_details, inlet_socket_addr)
            })
            .fold(submenu_builder, |menu, submenu| menu.item(&submenu));
        submenus.push(
            submenu_builder
                .build()
                .expect("cannot build accepted submenu"),
        );
    }
    submenus
}

fn accepted_invite_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    _invitation_id: &str,
    access_details: &ServiceAccessDetails,
    inlet_socket_addr: Option<&SocketAddr>,
) -> Submenu<R> {
    let service_name = access_details
        .service_name()
        .unwrap_or_else(|_| "Unknown service name".to_string());
    let mut submenu_builder = SubmenuBuilder::new(app_handle, &service_name);
    submenu_builder = match inlet_socket_addr {
        Some(s) => submenu_builder.items(&[
            &IconMenuItemBuilder::new(format!("Available at: {s}"))
                .enabled(false)
                .native_icon(NativeIcon::StatusAvailable)
                .build(app_handle),
            &IconMenuItemBuilder::with_id(
                format!("invitation-accepted-copy-{s}"),
                format!("Copy {s}"),
            )
            .icon(Icon::Raw(
                include_bytes!("../../icons/clipboard2.png").to_vec(),
            ))
            .build(app_handle),
        ]),
        None => submenu_builder.item(
            &IconMenuItemBuilder::new("Not connected")
                .native_icon(NativeIcon::StatusUnavailable)
                .build(app_handle),
        ),
    };
    submenu_builder
        .build()
        .expect("cannot build accepted invitation submenu")
}

pub fn process_tray_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    event: &MenuEvent,
) -> tauri::Result<()> {
    let id = event.id.as_ref();
    if id.starts_with("invitation-") {
        dispatch_click_event(app, id)
    } else {
        Ok(())
    }
}

fn dispatch_click_event<R: Runtime>(app: &AppHandle<R>, id: &str) -> tauri::Result<()> {
    let segments = id
        .splitn(4, '-')
        .skip_while(|segment| segment == &"invitation")
        .collect::<Vec<&str>>();
    match segments.as_slice() {
        ["create", "for", outlet_socket_addr] => on_create(app, outlet_socket_addr),
        ["received", "accept", id] => on_accept(app, id),
        ["accepted", "copy", socket_address] => on_copy(app, socket_address),
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
    spawn(async move {
        let _ = super::commands::accept_invitation(invite_id, app_handle)
            .await
            .map_err(|e| error!(%e, "Failed to accept invitation"));
    });

    Ok(())
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
            .max_inner_size(450.0, 350.0)
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

fn on_copy<R: Runtime>(_app: &AppHandle<R>, socket_address: &str) -> tauri::Result<()> {
    debug!(?socket_address, "Copying TCP inlet address");
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(socket_address);
    }
    Ok(())
}
