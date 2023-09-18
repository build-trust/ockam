use crate::app::ModelState;
use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::NodeManagerWorker;
use std::sync::Arc;
use tracing::{debug, error};

impl ModelState {
    pub fn add_tcp_outlet(&mut self, status: OutletStatus) {
        self.tcp_outlets.push(status);
    }

    pub fn delete_tcp_outlet(&mut self, alias: &str) {
        self.tcp_outlets.retain(|x| x.alias != alias);
    }

    pub fn get_tcp_outlets(&self) -> &[OutletStatus] {
        &self.tcp_outlets
    }
}

pub(crate) async fn load_model_state(
    context: Arc<Context>,
    node_manager_worker: &NodeManagerWorker,
    model_state: &ModelState,
    cli_state: &CliState,
) {
    if !cli_state.is_enrolled().unwrap_or(false) {
        return;
    }
    for tcp_outlet in model_state.get_tcp_outlets() {
        debug!(worker_addr = %tcp_outlet.worker_addr, "Restoring outlet");
        let _ = node_manager_worker
            .node_manager
            .create_outlet(
                &context,
                tcp_outlet.socket_addr,
                tcp_outlet.worker_addr.clone(),
                Some(tcp_outlet.alias.clone()),
                true,
            )
            .await
            .map_err(|e| {
                error!(
                    ?e,
                    worker_addr = %tcp_outlet.worker_addr,
                    "Failed to restore outlet"
                );
            });
    }
}
