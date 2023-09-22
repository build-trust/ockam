pub(crate) mod commands;
pub(crate) mod events;
pub(super) mod plugin;
pub(crate) mod state;
mod tray_menu;

use std::net::SocketAddr;
pub(crate) use tray_menu::*;

use crate::app::{AppState, NODE_NAME};
use crate::Error;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::cloud::share::CreateServiceInvitation;
use ockam_api::identity::EnrollmentTicket;
use tauri::{AppHandle, Manager, Runtime, State};

pub(crate) async fn build_args_for_create_service_invitation<R: Runtime>(
    app_handle: &AppHandle<R>,
    outlet_socket_addr: &SocketAddr,
    recipient_email: &str,
    enrollment_ticket: EnrollmentTicket,
) -> crate::Result<CreateServiceInvitation> {
    let app_state: State<'_, AppState> = app_handle.state();
    let cli_state = app_state.state().await;

    let service_route = app_state
        .model(|m| {
            m.tcp_outlets
                .iter()
                .find(|o| o.socket_addr == *outlet_socket_addr)
                .map(|o| o.worker_address())
        })
        .await
        .ok_or::<Error>("outlet should exist".into())??;
    let project = cli_state.projects.default()?;

    Ok(CreateServiceInvitation::new(
        &cli_state,
        None,
        project.name(),
        recipient_email,
        NODE_NAME,
        service_route.to_string().as_str(),
        enrollment_ticket,
    )
    .await?)
}
