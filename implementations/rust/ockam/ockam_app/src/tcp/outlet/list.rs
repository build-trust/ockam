use tauri::{AppHandle, Manager, Wry};

use ockam_api::nodes::models::portal::OutletList;

use crate::app::AppState;
use crate::Result;

/// List TCP outlets of the default node.
pub async fn list(app: &AppHandle<Wry>) -> Result<OutletList> {
    let state = app.state::<AppState>();
    let node_manager = state.node_manager();
    Ok(node_manager.list_outlets().await)
}
