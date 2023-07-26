use miette::miette;
use tauri::{AppHandle, Manager, Wry};
use tracing::log::info;

use crate::app::AppState;
use crate::Result;

/// Reset the project.
/// This function removes all persisted state
/// So that the user must enroll again in order to be able to access a project
pub async fn reset(app: &AppHandle<Wry>) -> Result<()> {
    let app_state = app.state::<AppState>();
    let result = app_state.reset().await;
    info!("Application state recreated");
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    result.map_err(|e| miette!(e).into())
}
