use miette::IntoDiagnostic;
use std::net::SocketAddr;
use std::str::FromStr;
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{debug, error, info, warn};

use ockam_api::address::{extract_address_value, get_free_address};
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::share::{AcceptInvitation, InvitationWithAccess};
use ockam_api::{
    cloud::share::{InvitationListKind, ListInvitations},
    nodes::models::portal::OutletStatus,
};

use crate::app::{AppState, NODE_NAME, PROJECT_NAME};
use crate::cli::cli_bin;
use crate::projects::commands::{create_enrollment_ticket, list_projects_with_admin};

use super::{
    events::REFRESHED_INVITATIONS,
    state::{InvitationState, SyncState},
};

// At time of writing, tauri::command requires pub not pub(crate)

#[tauri::command]
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
    let state: State<'_, AppState> = app.state();
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

    let node_manager_worker = state.node_manager_worker().await;
    let res = node_manager_worker
        .create_service_invitation(
            &state.context(),
            invite_args,
            &state.controller_address(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    debug!(?res, "invitation sent");
    app.trigger_global(super::events::REFRESH_INVITATIONS, None);
    Ok(())
}

#[tauri::command]
pub async fn list_invitations<R: Runtime>(app: AppHandle<R>) -> tauri::Result<InvitationState> {
    let state: State<'_, SyncState> = app.state();
    let reader = state.read().await;
    Ok((*reader).clone())
}

// TODO: move into shared_service module tree
#[tauri::command]
pub async fn list_outlets<R: Runtime>(app: AppHandle<R>) -> tauri::Result<Vec<OutletStatus>> {
    let state: State<'_, AppState> = app.state();
    let outlets = state.tcp_outlet_list().await;
    debug!(?outlets);
    Ok(outlets)
}

#[tauri::command]
pub async fn refresh_invitations<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("refreshing invitations");
    let state: State<'_, AppState> = app.state();
    if !state.is_enrolled().await.unwrap_or(false) {
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
    debug!(?invitations);
    {
        let invitation_state: State<'_, SyncState> = app.state();
        let mut writer = invitation_state.write().await;
        *writer = invitations.into();
    }
    refresh_inlets(&app).await.map_err(|e| e.to_string())?;
    app.trigger_global(REFRESHED_INVITATIONS, None);
    Ok(())
}

async fn refresh_inlets<R: Runtime>(app: &AppHandle<R>) -> crate::Result<()> {
    debug!("Refreshing inlets");
    let invitations_state: State<'_, SyncState> = app.state();
    let reader = invitations_state.read().await;
    if reader.accepted.is_empty() {
        return Ok(());
    }
    let app_state: State<'_, AppState> = app.state();
    let cli_state = app_state.state().await;
    let cli_bin = cli_bin()?;
    for invitation in &reader.accepted {
        match InletDataFromInvitation::new(&cli_state, invitation) {
            Ok(i) => match i {
                Some(i) => {
                    if let Ok(node) = cli_state.nodes.get(&i.local_node_name) {
                        if node.is_running() {
                            debug!(node = %i.local_node_name, "Node already running");
                            continue;
                        }
                    }
                    debug!(node = %i.local_node_name, "Deleting node");
                    let _ = duct::cmd!(
                        &cli_bin,
                        "node",
                        "delete",
                        "--quiet",
                        "--yes",
                        &i.local_node_name
                    )
                    .run();
                    let inlet_socket_addr = create_inlet(&i).await?;
                    if let Some(tray_item) = app.tray_handle().try_get_item(&format!(
                        "invitation-accepted-connect-{}",
                        invitation.invitation.id
                    )) {
                        let _ = tray_item.set_title(inlet_socket_addr.to_string());
                    }
                }
                None => {
                    warn!("Invalid invitation data");
                    continue;
                }
            },
            Err(err) => {
                warn!(%err, "Failed to parse invitation data");
                continue;
            }
        }
    }
    info!("Inlets refreshed");
    Ok(())
}

async fn create_inlet(inlet_data: &InletDataFromInvitation) -> crate::Result<SocketAddr> {
    debug!(?inlet_data, "Creating tcp-inlet for accepted invitation");
    let InletDataFromInvitation {
        local_node_name,
        service_name,
        service_route,
        enrollment_ticket_hex,
    } = inlet_data;
    let from = get_free_address()?;
    let from_str = from.to_string();
    let run_cmd_template = indoc::formatdoc! {
        r#"
        nodes:
          {local_node_name}:
            enrollment-ticket: {enrollment_ticket_hex}
            tcp-inlets:
              {service_name}:
                from: {from_str}
                to: {service_route}
        "#
    };
    duct::cmd!(cli_bin()?, "run", "--inline", run_cmd_template)
        .env("QUIET", "1")
        .run()
        .map_err(|e| {
            error!(%e, enrollment_ticket=enrollment_ticket_hex, "Could not create a tcp-inlet for the accepted invitation");
            e
        })?;
    info!(
        from = from_str,
        to = service_route,
        "Created tcp-inlet for accepted invitation"
    );
    Ok(from)
}

#[derive(Debug)]
struct InletDataFromInvitation {
    pub local_node_name: String,
    pub service_name: String,
    pub service_route: String,
    pub enrollment_ticket_hex: String,
}

impl InletDataFromInvitation {
    pub fn new(
        cli_state: &CliState,
        invitation: &InvitationWithAccess,
    ) -> crate::Result<Option<Self>> {
        match &invitation.service_access_details {
            Some(d) => {
                let service_name = extract_address_value(&d.shared_node_route)?;
                let enrollment_ticket_hex = d.enrollment_ticket.clone();
                let enrollment_ticket = d.enrollment_ticket()?;
                if let Some(project) = enrollment_ticket.project() {
                    let mut project = Project::from(project.clone());

                    // Rename project so that the local user can receive multiple invitations from different
                    // projects called "default" while keeping access to its own "default" project.
                    // Te node created here is meant to only serve the tcp-inlet and only has to resolve
                    // the `/project/{id}` project to create the needed secure-channel.
                    project.name = project.id.clone();
                    let project = cli_state
                        .projects
                        .overwrite(project.name.clone(), project)?;

                    let project_id = project.id();
                    let local_node_name = format!("ockam_app_{project_id}_{service_name}");
                    let service_route = format!(
                        "/project/{project_id}/service/forward_to_{}/secure/api/service/{service_name}",
                        NODE_NAME
                    );

                    Ok(Some(Self {
                        local_node_name,
                        service_name,
                        service_route,
                        enrollment_ticket_hex,
                    }))
                } else {
                    warn!(?invitation, "No project data found in enrollment ticket");
                    Ok(None)
                }
            }
            None => {
                warn!(
                    ?invitation,
                    "No service details found in accepted invitation"
                );
                Ok(None)
            }
        }
    }
}
