use std::sync::Arc;

use tauri::{async_runtime::RwLock, AppHandle, Manager, Runtime, State};
use tracing::{debug, error, info, trace, warn};

use ockam_api::cloud::project::Projects;
use ockam_api::{cli_state::StateDirTrait, cloud::project::Project, identity::EnrollmentTicket};

use crate::app::AppState;
use crate::projects::error::Error::{InternalFailure, ListingFailed, StateSaveFailed};

use super::error::{Error, Result};
use super::State as ProjectState;

// Store the user's admin projects
pub type SyncAdminProjectsState = Arc<RwLock<ProjectState>>;

pub(crate) async fn create_enrollment_ticket<R: Runtime>(
    project_id: String,
    app: AppHandle<R>,
) -> Result<EnrollmentTicket> {
    let app_state: State<'_, AppState> = app.state();
    let projects_state: State<'_, SyncAdminProjectsState> = app.state();
    let projects = projects_state.read().await;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?;

    debug!(?project_id, "Creating enrollment ticket via CLI");
    // TODO: How might this degrade for users who have multiple spaces and projects?
    let background_node_client = app_state.background_node_client().await;
    let hex_encoded_ticket = background_node_client
        .projects()
        .ticket(&project.name)
        .await
        .map_err(|_| Error::EnrollmentTicketFailed)?;
    serde_json::from_slice(&hex::decode(hex_encoded_ticket).map_err(|err| {
        error!(?err, "Could not hex-decode enrollment ticket");
        Error::EnrollmentTicketDecodeFailed
    })?)
    .map_err(|err| {
        error!(?err, "Could not JSON-decode enrollment ticket");
        Error::EnrollmentTicketDecodeFailed
    })
}

pub(crate) async fn refresh_projects<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    info!("Refreshing projects");
    let state: State<'_, AppState> = app.state();
    if !state.is_enrolled().await.unwrap_or(false) {
        return Ok(());
    }
    let email = match state.user_email().await {
        Ok(email) => email,
        Err(_) => {
            warn!("User info is not available");
            return Ok(());
        }
    };

    let controller = state
        .controller()
        .await
        .map_err(|e| InternalFailure(e.to_string()))?;
    let projects = controller
        .list_projects(&state.context())
        .await
        .map_err(ListingFailed)?
        .success()
        .map_err(ListingFailed)?
        .into_iter()
        .filter(|p| p.has_admin_with_email(&email))
        .collect::<Vec<Project>>();
    debug!("Projects fetched");
    trace!(?projects);

    let cli_projects = state.state().await.projects;
    for project in &projects {
        cli_projects
            .overwrite(&project.name, project.clone())
            .map_err(|_| StateSaveFailed)?;
    }

    let project_state: State<'_, SyncAdminProjectsState> = app.state();
    let mut writer = project_state.write().await;
    *writer = projects;

    app.trigger_global(super::events::REFRESHED_PROJECTS, None);
    Ok(())
}
