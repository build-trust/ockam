use std::net::SocketAddr;

use tauri::{AppHandle, Manager, Runtime, State};

use ockam_api::cli_state::StateDirTrait;
use ockam_api::cloud::share::CreateServiceInvitation;
use ockam_api::identity::EnrollmentTicket;
pub(crate) use tray_menu::*;

use crate::app::{AppState, NODE_NAME};
use crate::error::Error;

pub(crate) mod commands;
pub(crate) mod events;
pub(super) mod plugin;
mod state;
mod tray_menu;

pub(crate) async fn build_args_for_create_service_invitation<R: Runtime>(
    app_handle: &AppHandle<R>,
    outlet_socket_addr: &SocketAddr,
    recipient_email: &str,
    enrollment_ticket: EnrollmentTicket,
) -> crate::Result<CreateServiceInvitation> {
    let app_state: State<'_, AppState> = app_handle.state();
    let cli_state = app_state.state().await;

    let outlet_status = app_state
        .model(|m| m.get_outlet_status_by_socket_addr(outlet_socket_addr))
        .await
        .ok_or::<Error>("outlet should exist".into())?;
    let project = cli_state.projects.default()?;
    let service_name = outlet_status
        .worker_address()?
        .to_string()
        .replace("/service/", "");

    Ok(CreateServiceInvitation::new(
        &cli_state,
        None,
        project.name(),
        recipient_email,
        NODE_NAME,
        service_name.as_str(),
        outlet_status.worker_address()?.to_string().as_str(),
        enrollment_ticket,
    )
    .await?)
}
