use ockam_api::nodes::models::portal::OutletStatus;

use crate::app::AppState;
use crate::Result;

/// List TCP outlets of the default node.
pub async fn tcp_outlet_list(app_state: &AppState) -> Result<Vec<OutletStatus>> {
    let node_manager = app_state.node_manager();
    Ok(node_manager.list_outlets().await.list)
}
