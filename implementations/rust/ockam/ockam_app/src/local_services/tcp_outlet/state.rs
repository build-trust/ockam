use crate::app::ModelState;
use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::models::portal::ServiceStatus;
use ockam_api::nodes::NodeManagerWorker;
use std::sync::Arc;
use tracing::error;

impl ModelState {
    pub fn add_tcp_outlet(&mut self, status: ServiceStatus) {
        self.tcp_outlets.push(status);
    }

    pub fn get_tcp_outlets(&self) -> &[ServiceStatus] {
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
    let mut node_manager = node_manager_worker.inner().write().await;
    for tcp_outlet in model_state.get_tcp_outlets() {
        let _ = node_manager
            .create_outlet(
                &context,
                tcp_outlet.socket_addr,
                tcp_outlet.worker_addr.clone(),
                None,
                true,
            )
            .await
            .map_err(|e| {
                error!(
                    ?e,
                    "failed to create outlet with tcp addr {}", tcp_outlet.socket_addr
                );
            });
    }
}
