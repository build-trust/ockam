use crate::app::ModelState;
use ockam::Context;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::NodeManagerWorker;
use std::sync::Arc;

impl ModelState {
    pub fn add_tcp_outlet(&mut self, status: OutletStatus) {
        self.tcp_outlets.push(status);
    }

    pub fn get_tcp_outlets(&self) -> &[OutletStatus] {
        &self.tcp_outlets
    }
}

pub(crate) async fn load_model_state(
    context: Arc<Context>,
    node_manager_worker: &NodeManagerWorker,
    model_state: &ModelState,
) {
    let mut node_manager = node_manager_worker.inner().write().await;
    for tcp_outlet in model_state.get_tcp_outlets() {
        node_manager
            .create_outlet(
                &context,
                tcp_outlet.tcp_addr.clone(),
                tcp_outlet.worker_addr.clone(),
                None,
                true,
            )
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "failed to create outlet with tcp addr {}",
                    tcp_outlet.tcp_addr
                )
            });
    }
}
