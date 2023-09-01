use miette::IntoDiagnostic;
use std::net::SocketAddr;
use std::str::FromStr;
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{debug, info, trace};

use crate::app::{AppState, PROJECT_NAME};
use crate::invitations::state::SyncState;
use crate::projects::commands::{create_enrollment_ticket, list_projects_with_admin};
use ockam_api::cloud::share::{AcceptInvitation, CreateServiceInvitation};
use ockam_api::cloud::share::{InvitationListKind, ListInvitations};

use super::events::REFRESHED_INVITATIONS;

pub async fn accept_invitation<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), String> {
    accept_invitation_impl(id, &app)
        .await
        .map_err(|e| e.to_string())?;
    app.trigger_global(super::events::REFRESH_INVITATIONS, None);
    Ok(())
}

async fn accept_invitation_impl<R: Runtime>(id: String, app: &AppHandle<R>) -> crate::Result<()> {
    info!(?id, "accepting invitation");
    let state: State<'_, AppState> = app.state();
    let node_manager_worker = state.node_manager_worker().await;
    let res = node_manager_worker
        .accept_invitation(
            &state.context(),
            AcceptInvitation { id },
            &state.controller_address(),
            None,
        )
        .await?;
    debug!(?res);
    Ok(())
}

#[tauri::command]
pub async fn create_service_invitation<R: Runtime>(
    recipient_email: String,
    outlet_socket_addr: String,
    app: AppHandle<R>,
) -> Result<(), String> {
    info!(
        ?recipient_email,
        ?outlet_socket_addr,
        "creating service invitation"
    );

    let projects = list_projects_with_admin(app.clone()).await?;
    let project_id = projects
        .iter()
        .find(|p| p.name == *PROJECT_NAME)
        .map(|p| p.id.to_owned())
        .ok_or_else(|| "could not find default project".to_string())?;
    let enrollment_ticket = create_enrollment_ticket(project_id, app.clone())
        .await
        .map_err(|e| e.to_string())?;

    let socket_addr = SocketAddr::from_str(outlet_socket_addr.as_str())
        .into_diagnostic()
        .map_err(|e| format!("Cannot parse the outlet address as a socket address: {e}"))?;
    let invite_args = super::build_args_for_create_service_invitation(
        &app,
        &socket_addr,
        &recipient_email,
        enrollment_ticket,
    )
    .await
    .map_err(|e| e.to_string())?;

    // send the invitation asynchronously to avoid blocking the application waiting for a result
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let _ = send_invitation(invite_args, &app_clone).await;
        app_clone.trigger_global(super::events::REFRESH_INVITATIONS, None);
    });
    Ok(())
}

async fn send_invitation<R: Runtime>(
    invite_args: CreateServiceInvitation,
    app: &AppHandle<R>,
) -> crate::Result<()> {
    let state: State<'_, AppState> = app.state();
    let node_manager_worker = state.node_manager_worker().await;
    let res = node_manager_worker
        .create_service_invitation(
            &state.context(),
            invite_args,
            &state.controller_address(),
            None,
        )
        .await
        .map_err(|e| e.to_string());
    debug!(?res, "invitation sent");
    Ok(())
}

pub async fn refresh_invitations<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    debug!("refreshing invitations");
    let state: State<'_, AppState> = app.state();
    if !state.is_enrolled().await.unwrap_or(false) {
        debug!("not enrolled, skipping invitations refresh");
        return Ok(());
    }
    let node_manager_worker = state.node_manager_worker().await;
    let invitations = node_manager_worker
        .list_shares(
            &state.context(),
            ListInvitations {
                kind: InvitationListKind::All,
            },
            &state.controller_address(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    trace!(?invitations);
    {
        let invitation_state: State<'_, SyncState> = app.state();
        let mut writer = invitation_state.write().await;
        writer.replace_by(invitations);
    }
    app.trigger_global(REFRESHED_INVITATIONS, None);
    Ok(())
}
