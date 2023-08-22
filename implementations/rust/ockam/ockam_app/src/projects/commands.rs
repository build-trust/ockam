use std::sync::Arc;

use tauri::{async_runtime::RwLock, AppHandle, Manager, Runtime, State};
use tracing::{debug, error, info};

use ockam_api::{cli_state::StateDirTrait, cloud::project::Project, identity::EnrollmentTicket};
use ockam_command::util::api::CloudOpts;

use super::error::{Error, Result};
use super::State as ProjectState;
use crate::app::AppState;
use crate::cli::cli_bin;

type SyncState = Arc<RwLock<ProjectState>>;

// Matches backend default of 14 days
const DEFAULT_ENROLLMENT_TICKET_TTL: usize = 60 * 60 * 24 * 14;

// At time of writing, tauri::command requires pub not pub(crate)

#[tauri::command]
pub async fn create_enrollment_ticket<R: Runtime>(
    project_id: String,
    app: AppHandle<R>,
) -> Result<EnrollmentTicket> {
    let projects = list_projects_with_admin(app).await?;
    let project = projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| Error::ProjectNotFound(project_id.to_owned()))?;

    debug!(?project_id, "creating enrollment ticket via CLI");
    // TODO: Issue enrollment ticket using in-memory code instead of subshell
    // TODO: How might this degrade for users who have multiple spaces and projects?
    let bin = cli_bin().map_err(|_| Error::OckamCommandInvalid)?;
    let hex_encoded_ticket = duct::cmd!(
        bin,
        "project",
        "ticket",
        "--quiet",
        "--project",
        project.name.clone(),
        // TODO: rename to expires-in after https://github.com/build-trust/ockam/pull/5677
        "--ticket-ttl",
        DEFAULT_ENROLLMENT_TICKET_TTL.to_string(),
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

pub(crate) async fn list_projects_with_admin<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<Project>> {
    let app_state: State<'_, AppState> = app.state();
    let user_email = app_state.user_email().await.unwrap_or_default();
    let state: State<'_, SyncState> = app.state();
    let reader = state.read().await;
    Ok(reader
        .iter()
        .filter(|project| project.has_admin_with_email(&user_email))
        .cloned()
        .collect())
}

#[tauri::command]
pub async fn list_projects<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Project>> {
    let state: State<'_, SyncState> = app.state();
    let reader = state.read().await;
    debug!(projects = ?reader);
    Ok((*reader).clone())
}

#[tauri::command]
pub async fn refresh_projects<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    info!("refreshing projects");
    let state: State<'_, AppState> = app.state();
    let node_manager_worker = state.node_manager_worker().await;
    let projects = node_manager_worker
        .list_projects(&state.context(), &CloudOpts::route())
        .await
        .map_err(Error::ListingFailed)?;
    debug!(?projects);

    let cli_projects = state.state().await.projects;
    for project in &projects {
        cli_projects
            .overwrite(&project.name, project.clone())
            .map_err(|_| Error::StateSaveFailed)?;
    }

    let project_state: State<'_, SyncState> = app.state();
    let mut writer = project_state.write().await;
    *writer = projects;

    app.trigger_global(super::events::REFRESHED_PROJECTS, None);
    Ok(())
}
