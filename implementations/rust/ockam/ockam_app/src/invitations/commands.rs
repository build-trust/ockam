use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use miette::IntoDiagnostic;
use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{debug, info, trace, warn};

use ockam_api::address::get_free_address;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::share::InvitationListKind;
use ockam_api::cloud::share::{CreateServiceInvitation, InvitationWithAccess, Invitations};

use crate::app::events::system_tray_on_update;
use crate::app::{AppState, PROJECT_NAME};
use crate::background_node::BackgroundNodeClient;
use crate::invitations::state::{Inlet, ReceivedInvitationStatus};
use crate::projects::commands::{create_enrollment_ticket, SyncAdminProjectsState};
use crate::shared_service::relay::RELAY_NAME;

use super::{events::REFRESHED_INVITATIONS, state::SyncInvitationsState};

pub async fn accept_invitation<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), String> {
    accept_invitation_impl(id, &app)
        .await
        .map_err(|e| e.to_string())?;
    app.trigger_global(super::events::REFRESH_INVITATIONS, None);
    Ok(())
}

async fn accept_invitation_impl<R: Runtime>(id: String, app: &AppHandle<R>) -> crate::Result<()> {
    debug!(?id, "Accepting invitation");
    let app_state: State<'_, AppState> = app.state();

    if !app_state.is_enrolled().await? {
        debug!(?id, "Not enrolled, invitation can't be accepted");
        return Ok(());
    }

    let invitation_state: State<'_, SyncInvitationsState> = app.state();
    // Update the invitation status to Accepting if it's not already being processed.
    // Otherwise, return early.
    {
        let mut writer = invitation_state.write().await;
        match writer.received.status.iter_mut().find(|x| x.0 == id) {
            None => {
                writer
                    .received
                    .status
                    .push((id.clone(), ReceivedInvitationStatus::Accepting));
                system_tray_on_update(app);
            }
            Some((i, s)) => {
                return match s {
                    ReceivedInvitationStatus::Accepting => {
                        debug!(?i, "Invitation is being processed");
                        Ok(())
                    }
                    ReceivedInvitationStatus::Accepted => {
                        debug!(?i, "Invitation was already accepted");
                        Ok(())
                    }
                }
            }
        }
    }

    let controller = app_state.controller().await.into_diagnostic()?;
    let res = controller
        .accept_invitation(&app_state.context(), id.clone())
        .await?;

    // Update the invitation status to Accepted
    {
        let mut writer = invitation_state.write().await;
        if let Some(x) = writer.received.status.iter_mut().find(|x| x.0 == id) {
            x.1 = ReceivedInvitationStatus::Accepted;
            system_tray_on_update(app);
        }
    }

    debug!(?res);
    info!(?id, "Invitation accepted");
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
    let state: State<'_, SyncAdminProjectsState> = app.state();
    let projects = state.read().await;
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

    let controller = state.controller().await.into_diagnostic()?;
    let CreateServiceInvitation {
        expires_at,
        project_id,
        recipient_email,
        project_identity,
        project_route,
        project_authority_identity,
        project_authority_route,
        shared_node_identity,
        shared_node_route,
        enrollment_ticket,
    } = invite_args;
    let res = controller
        .create_service_invitation(
            &state.context(),
            expires_at,
            project_id,
            recipient_email,
            project_identity,
            project_route,
            project_authority_identity,
            project_authority_route,
            shared_node_identity,
            shared_node_route,
            enrollment_ticket,
        )
        .await
        .map_err(|e| e.to_string())?;
    debug!(?res, "invitation sent");
    Ok(())
}

pub async fn refresh_invitations<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    debug!("Refreshing invitations");
    let invitations = {
        let state: State<'_, AppState> = app.state();
        if !state.is_enrolled().await.unwrap_or(false) {
            debug!("not enrolled, skipping invitations refresh");
            return Ok(());
        }
        let controller = state.controller().await.map_err(|e| e.to_string())?;
        let invitations = controller
            .list_invitations(&state.context(), InvitationListKind::All)
            .await
            .map_err(|e| e.to_string())?;
        debug!("Invitations fetched");
        trace!(?invitations);
        invitations
    };
    {
        let invitation_state: State<'_, SyncInvitationsState> = app.state();
        let mut writer = invitation_state.write().await;
        writer.replace_by(invitations);
    }
    refresh_inlets(&app).await.map_err(|e| e.to_string())?;
    app.trigger_global(REFRESHED_INVITATIONS, None);
    Ok(())
}

async fn refresh_inlets<R: Runtime>(app: &AppHandle<R>) -> crate::Result<()> {
    debug!("Refreshing inlets");
    let app_state: State<'_, AppState> = app.state();
    let invitation_state: State<'_, SyncInvitationsState> = app.state();
    let mut invitations_state = invitation_state.write().await;
    if invitations_state.accepted.invitations.is_empty() {
        debug!("No accepted invitations, skipping inlets refresh");
        return Ok(());
    }

    let cli_state = app_state.state().await;
    let background_node_client = app_state.background_node_client().await;
    let mut running_inlets = vec![];
    for invitation in &invitations_state.accepted.invitations {
        match InletDataFromInvitation::new(
            &cli_state,
            invitation,
            &invitations_state.accepted.inlets,
        ) {
            Ok(i) => match i {
                Some(mut i) => {
                    if !i.enabled {
                        debug!(node = %i.local_node_name, "TCP inlet is disabled by the user, skipping");
                        continue;
                    }

                    debug!(node = %i.local_node_name, "Checking node status");
                    if let Ok(node) = cli_state.nodes.get(&i.local_node_name) {
                        if node.is_running() {
                            debug!(node = %i.local_node_name, "Node already running");
                            if let Ok(inlet) = background_node_client
                                .inlets()
                                .show(&i.local_node_name, &i.service_name)
                                .await
                            {
                                i.socket_addr = Some(inlet.bind_addr.parse()?);
                                running_inlets.push((invitation.invitation.id.clone(), i));
                                continue;
                            }
                        }
                    }
                    background_node_client
                        .nodes()
                        .delete(&i.local_node_name)
                        .await?;
                    match create_inlet(background_node_client.clone(), &i).await {
                        Ok(socket_addr) => {
                            i.socket_addr = Some(socket_addr);
                            running_inlets.push((invitation.invitation.id.clone(), i));
                        }
                        Err(err) => {
                            warn!(%err, node = %i.local_node_name, "Failed to create TCP inlet for accepted invitation");
                        }
                    }
                }
                None => {
                    warn!("Invalid invitation data");
                }
            },
            Err(err) => {
                warn!(%err, "Failed to parse invitation data");
            }
        }
    }
    for (invitation_id, i) in running_inlets {
        invitations_state
            .accepted
            .inlets
            .insert(invitation_id, Inlet::new(i)?);
    }
    info!("Inlets refreshed");
    Ok(())
}

/// Create the tcp-inlet for the accepted invitation
/// Returns the inlet SocketAddr
async fn create_inlet(
    background_node_client: Arc<dyn BackgroundNodeClient>,
    inlet_data: &InletDataFromInvitation,
) -> crate::Result<SocketAddr> {
    debug!(service_name = ?inlet_data.service_name, "Creating TCP inlet for accepted invitation");
    let InletDataFromInvitation {
        enabled,
        local_node_name,
        service_name,
        service_route,
        enrollment_ticket_hex,
        socket_addr,
    } = inlet_data;
    if !enabled {
        return Err("TCP inlet is disabled by the user".into());
    }
    let from = match socket_addr {
        Some(socket_addr) => *socket_addr,
        None => get_free_address()?,
    };
    if let Some(enrollment_ticket_hex) = enrollment_ticket_hex {
        background_node_client
            .projects()
            .enroll(local_node_name, enrollment_ticket_hex)
            .await?;
    }
    background_node_client
        .nodes()
        .create(local_node_name)
        .await?;
    background_node_client
        .inlets()
        .create(local_node_name, &from, service_route, service_name)
        .await?;
    Ok(from)
}

pub(crate) async fn disconnect_tcp_inlet<R: Runtime>(
    app: AppHandle<R>,
    invitation_id: &str,
) -> crate::Result<()> {
    let app_state: State<'_, AppState> = app.state();
    let background_node_client = app_state.background_node_client().await;
    let invitation_state: State<'_, SyncInvitationsState> = app.state();
    let mut writer = invitation_state.write().await;
    if let Some(inlet) = writer.accepted.inlets.get_mut(invitation_id) {
        if !inlet.enabled {
            debug!(node = %inlet.node_name, alias = %inlet.alias, "TCP inlet was already disconnected");
            return Ok(());
        }
        inlet.disable();
        background_node_client
            .inlets()
            .delete(&inlet.node_name, &inlet.alias)
            .await?;
    }
    Ok(())
}

pub(crate) async fn enable_tcp_inlet<R: Runtime>(
    app: AppHandle<R>,
    invitation_id: &str,
) -> crate::Result<()> {
    let invitation_state: State<'_, SyncInvitationsState> = app.state();
    let mut writer = invitation_state.write().await;
    if let Some(inlet) = writer.accepted.inlets.get_mut(invitation_id) {
        if inlet.enabled {
            debug!(node = %inlet.node_name, alias = %inlet.alias, "TCP inlet was already enabled");
            return Ok(());
        }
        inlet.enable();
        app.trigger_global(super::events::REFRESH_INVITATIONS, None);
        info!(node = %inlet.node_name, alias = %inlet.alias, "Enabled TCP inlet");
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct InletDataFromInvitation {
    pub enabled: bool,
    pub local_node_name: String,
    pub service_name: String,
    pub service_route: String,
    pub enrollment_ticket_hex: Option<String>,
    pub socket_addr: Option<SocketAddr>,
}

impl InletDataFromInvitation {
    pub fn new(
        cli_state: &CliState,
        invitation: &InvitationWithAccess,
        inlets: &HashMap<String, Inlet>,
    ) -> crate::Result<Option<Self>> {
        match &invitation.service_access_details {
            Some(d) => {
                let service_name = d.service_name()?;
                let mut enrollment_ticket = d.enrollment_ticket()?;
                // The enrollment ticket contains the project data.
                // We need to replace the project name on the enrollment ticket with the project id,
                // so that, when using the enrollment ticket, there are no conflicts with the default project.
                // The node created when setting up the TCP inlet is meant to only serve that TCP inlet and
                // only has to resolve the `/project/{id}` project to create the needed secure-channel.
                if let Some(project) = enrollment_ticket.project.as_mut() {
                    project.name = project.id.clone();
                }
                let enrollment_ticket_hex = if invitation.invitation.is_expired()? {
                    None
                } else {
                    Some(enrollment_ticket.hex_encoded()?)
                };

                if let Some(project) = enrollment_ticket.project {
                    // At this point, the project name will be the project id.
                    let project = cli_state
                        .projects
                        .overwrite(project.name.clone(), Project::from(project.clone()))?;
                    assert_eq!(
                        project.name(),
                        project.id(),
                        "Project name should be the project id"
                    );

                    let project_id = project.id();
                    let local_node_name = format!("ockam_app_{project_id}_{service_name}");
                    let service_route = format!(
                        "/project/{project_id}/service/{}/secure/api/service/{service_name}",
                        *RELAY_NAME
                    );

                    let inlet = inlets.get(&invitation.invitation.id);
                    let enabled = inlet.map(|i| i.enabled).unwrap_or(true);
                    let socket_addr = inlet.map(|i| i.socket_addr);

                    Ok(Some(Self {
                        enabled,
                        local_node_name,
                        service_name,
                        service_route,
                        enrollment_ticket_hex,
                        socket_addr,
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

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::OneTimeCode;
    use ockam_api::cloud::share::{
        ReceivedInvitation, RoleInShare, ServiceAccessDetails, ShareScope,
    };
    use ockam_api::config::lookup::ProjectLookup;
    use ockam_api::identity::EnrollmentTicket;

    #[test]
    fn test_inlet_data_from_invitation() {
        let cli_state = CliState::test().unwrap();
        let mut inlets = HashMap::new();
        let mut invitation = InvitationWithAccess {
            invitation: ReceivedInvitation {
                id: "invitation_id".to_string(),
                expires_at: "2020-09-12T15:07:14.00".to_string(),
                grant_role: RoleInShare::Admin,
                owner_email: "owner_email".to_string(),
                scope: ShareScope::Project,
                target_id: "target_id".to_string(),
            },
            service_access_details: None,
        };

        // InletDataFromInvitation will be none because `service_access_details` is none
        assert!(
            InletDataFromInvitation::new(&cli_state, &invitation, &inlets)
                .unwrap()
                .is_none()
        );

        invitation.service_access_details = Some(ServiceAccessDetails {
            project_identity: "I1234561234561234561234561234561234561234"
                .try_into()
                .unwrap(),
            project_route: "project_route".to_string(),
            project_authority_identity: "Iabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .try_into()
                .unwrap(),
            project_authority_route: "project_authority_route".to_string(),
            shared_node_identity: "I12ab34cd56ef12ab34cd56ef12ab34cd56ef12ab"
                .try_into()
                .unwrap(),
            shared_node_route: "shared_node_route".to_string(),
            enrollment_ticket: EnrollmentTicket::new(
                OneTimeCode::new(),
                Some(ProjectLookup {
                    node_route: None,
                    id: "project_identity".to_string(),
                    name: "project_name".to_string(),
                    identity_id: None,
                    authority: None,
                    okta: None,
                }),
                None,
            )
            .hex_encoded()
            .unwrap(),
        });

        // Validate the inlet data, with no prior inlet data
        let inlet_data = InletDataFromInvitation::new(&cli_state, &invitation, &inlets)
            .unwrap()
            .unwrap();
        assert!(inlet_data.socket_addr.is_none());

        // Validate the inlet data, with prior inlet data
        inlets.insert(
            "invitation_id".to_string(),
            Inlet {
                node_name: "local_node_name".to_string(),
                alias: "alias".to_string(),
                socket_addr: "127.0.0.1:1000".parse().unwrap(),
                enabled: true,
            },
        );
        let inlet_data = InletDataFromInvitation::new(&cli_state, &invitation, &inlets)
            .unwrap()
            .unwrap();
        assert!(inlet_data.socket_addr.is_some());
    }
}
