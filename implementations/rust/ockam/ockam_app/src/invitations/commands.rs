use tauri::{AppHandle, Manager, Runtime, State};
use tracing::{debug, error, info, warn};

use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::share::{AcceptInvitation, InvitationWithAccess};
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::{
    cloud::share::{InvitationListKind, ListInvitations},
    nodes::models::portal::OutletStatus,
};
use ockam_command::util::api::CloudOpts;
use ockam_command::util::{extract_address_value, get_free_address};

use crate::app::{AppState, NODE_NAME};
use crate::cli::cli_bin;
use crate::projects::commands::create_enrollment_ticket;

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
            &CloudOpts::route(),
            None,
        )
        .await?;
    debug!(?res);
    Ok(())
}

#[tauri::command]
pub async fn create_service_invitation<R: Runtime>(
    recipient_email: String,
    outlet_addr: String,
    app: AppHandle<R>,
) -> Result<(), String> {
    info!(
        ?recipient_email,
        ?outlet_addr,
        "creating service invitation"
    );
    let state = app.state::<AppState>();
    let project_id = state
        .state()
        .await
        .projects
        .default()
        .map_err(|_| "could not load default project".to_string())?
        .id()
        .to_owned();
    let enrollment_ticket = create_enrollment_ticket(project_id, app.clone())
        .await
        .map_err(|e| e.to_string())?;
    let invite_args = super::build_args_for_create_service_invitation(
        &app,
        &outlet_addr,
        &recipient_email,
        enrollment_ticket,
    )
    .await
    .map_err(|e| e.to_string())?;

    let node_manager_worker = state.node_manager_worker().await;
    let res = node_manager_worker
        .create_service_invitation(&state.context(), invite_args, &CloudOpts::route(), None)
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
    let node_manager_worker = state.node_manager_worker().await;
    let invitations = node_manager_worker
        .list_shares(
            &state.context(),
            ListInvitations {
                kind: InvitationListKind::All,
            },
            &CloudOpts::route(),
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
    let invitations_state: State<'_, SyncState> = app.state();
    let reader = invitations_state.read().await;
    if reader.accepted.is_empty() {
        return Ok(());
    }
    let app_state: State<'_, AppState> = app.state();
    let cli_state = app_state.state().await;
    let accepted_invitations_by_node =
        reader
            .accepted
            .iter()
            .fold(std::collections::HashMap::new(), |mut acc, invitation| {
                let _ = InletDataFromInvitation::new(&cli_state, invitation).map(|d| {
                    d.map(|d| {
                        acc.entry(d.local_node_name.clone())
                            .or_insert_with(Vec::new)
                            .push(d);
                    })
                });
                acc
            });
    let cli_bin = cli_bin()?;
    for (node_name, invitations) in accepted_invitations_by_node {
        let mut node_inlets_to_delete = Vec::new();
        let node_inlets = {
            match duct::cmd!(
                &cli_bin,
                "tcp-inlet",
                "list",
                "--quiet",
                "--at",
                &node_name,
                "--output",
                "json"
            )
            .read()
            {
                Ok(json_res) => serde_json::from_str::<Vec<InletStatus>>(&json_res)?,
                Err(e) => {
                    warn!(?node_name, %e, "Could not list inlets for node");
                    // If the node doesn't exist, the command will return an error, so
                    // we initialize the inlets list to an empty vector.
                    Vec::new()
                }
            }
        };
        // Iterate the invitations and create the inlets if they don't exist
        for invitation_inlet_data in invitations {
            if let Some(inlet) = node_inlets
                .iter()
                // The inlets we create are named after the remote service name (i.e. the remote tcp-outlet name)
                .find(|inlet| inlet.alias == invitation_inlet_data.service_name)
            {
                // If the inlet already exists, mark it to remove it from the node
                node_inlets_to_delete.push(inlet);
            } else {
                create_inlet(&invitation_inlet_data).await?;
            }
        }
        // Remove the orphaned inlets (no matching invitations) from the node
        for inlet in node_inlets_to_delete {
            let _ = duct::cmd!(
                &cli_bin,
                "tcp-inlet",
                "delete",
                "--quiet",
                &inlet.alias,
                "--at",
                &node_name,
            )
            .run();
        }
    }
    Ok(())
}

async fn create_inlet(inlet_data: &InletDataFromInvitation) -> crate::Result<()> {
    let InletDataFromInvitation {
        local_node_name,
        service_name,
        service_route,
        enrollment_ticket_hex,
    } = inlet_data;
    let from = get_free_address()?.to_string(); // TODO: we should let the user pass this address
    let run_cmd_template = indoc::formatdoc! {
        r#"
        nodes:
          {local_node_name}:
            enrollment-ticket: {enrollment_ticket_hex}
            tcp-inlets:
              {service_name}:
                from: {from}
                to: {service_route}
        "#
    };
    duct::cmd!(cli_bin()?, "run", "--quiet", "--inline", run_cmd_template)
        .run()
        .map_err(|e| {
            error!(%e, enrollment_ticket=enrollment_ticket_hex, "Could not create a tcp-inlet for the accepted invitation");
            e
        })?;
    Ok(())
}

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
