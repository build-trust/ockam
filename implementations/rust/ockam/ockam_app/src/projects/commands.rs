use std::sync::Arc;

use tauri::{async_runtime::RwLock, AppHandle, Manager, Runtime, State};
use tracing::{debug, error, info, trace, warn};

use ockam_api::address::controller_route;
use ockam_api::{cli_state::StateDirTrait, cloud::project::Project, identity::EnrollmentTicket};

use super::error::{Error, Result};
use super::State as ProjectState;
use crate::app::AppState;
use crate::cli::cli_bin;

// Store the user's admin projects
pub type SyncAdminProjectsState = Arc<RwLock<ProjectState>>;

// Matches backend default of 14 days
const DEFAULT_ENROLLMENT_TICKET_EXPIRY: &str = "14d";

pub(crate) async fn create_enrollment_ticket<R: Runtime>(
    project_id: String,
    app: AppHandle<R>,
) -> Result<EnrollmentTicket> {
    let state: State<'_, SyncAdminProjectsState> = app.state();
    let projects = state.read().await;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?;

    debug!(?project_id, "creating enrollment ticket via CLI");
    // TODO: Issue enrollment ticket using in-memory code instead of subshell
    // TODO: How might this degrade for users who have multiple spaces and projects?
    let bin = cli_bin().map_err(|e| Error::OckamCommandInvalid(e.to_string()))?;
    let hex_encoded_ticket = duct::cmd!(
        bin,
        "project",
        "ticket",
        "--quiet",
        "--project",
        project.name.clone(),
        "--expires-in",
        DEFAULT_ENROLLMENT_TICKET_EXPIRY.to_string(),
        "--to",
        &format!("/project/{}", project.name)
    )
    .read()
    .map_err(|err| {
        error!(?err, "could not create enrollment ticket");
        Error::EnrollmentTicketFailed
    })?;
    serde_json::from_slice(&hex::decode(hex_encoded_ticket).map_err(|err| {
        error!(?err, "could not hex-decode enrollment ticket");
        Error::EnrollmentTicketDecodeFailed
    })?)
    .map_err(|err| {
        error!(?err, "could not JSON-decode enrollment ticket");
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

    let node_manager_worker = state.node_manager_worker().await;
    let projects = node_manager_worker
        .list_projects(&state.context(), &controller_route())
        .await
        .map_err(Error::ListingFailed)?
        .into_iter()
        .filter(|p| p.has_admin_with_email(&email))
        .collect::<Vec<Project>>();
    debug!("Projects fetched");
    trace!(?projects);

    let cli_projects = state.state().await.projects;
    for project in &projects {
        cli_projects
            .overwrite(&project.name, project.clone())
            .map_err(|_| Error::StateSaveFailed)?;
    }

    let project_state: State<'_, SyncAdminProjectsState> = app.state();
    let mut writer = project_state.write().await;
    *writer = projects;

    app.trigger_global(super::events::REFRESHED_PROJECTS, None);
    Ok(())
}
