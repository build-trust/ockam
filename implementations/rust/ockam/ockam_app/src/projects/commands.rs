use std::sync::Arc;

use tauri::{async_runtime::RwLock, AppHandle, Manager, Runtime, State};
use tracing::{debug, info};

use ockam_api::{cli_state::StateDirTrait, cloud::project::Project};
use ockam_command::util::api::CloudOpts;

use super::State as ProjectState;
use crate::app::AppState;

type SyncState = Arc<RwLock<ProjectState>>;

// At time of writing, tauri::command requires pub not pub(crate)

#[tauri::command]
pub async fn list_projects<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Project>, String> {
    let state: State<'_, SyncState> = app.state();
    let reader = state.read().await;
    Ok((*reader).clone())
}

#[tauri::command]
pub async fn refresh_projects<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    info!("refreshing projects");
    let state: State<'_, AppState> = app.state();
    let node_manager_worker = state.node_manager_worker().await;
    let projects = node_manager_worker
        .list_projects(&state.context(), &CloudOpts::route())
        .await
        .map_err(|e| e.to_string())?;
    debug!(?projects);

    let cli_projects = state.state().await.projects;
    for project in &projects {
        cli_projects
            .overwrite(&project.name, project.clone())
            .map_err(|e| format!("could not save project in cli state: {e}"))?;
    }

    let project_state: State<'_, SyncState> = app.state();
    let mut writer = project_state.write().await;
    *writer = projects;

    app.trigger_global(super::events::REFRESHED_PROJECTS, None);
    Ok(())
}
