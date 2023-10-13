use crate::state::{AppState, ModelState};
use ockam_api::cli_state::CliState;
use ockam_api::nodes::models::portal::OutletStatus;
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

impl AppState {
    pub(crate) async fn load_outlet_model_state(
        &self,
        model_state: &ModelState,
        cli_state: &CliState,
    ) {
        if !cli_state.is_enrolled().unwrap_or(false) {
            return;
        }
        let node_manager = self.node_manager().await;
        let context = self.context();
        for tcp_outlet in model_state.get_tcp_outlets() {
            debug!(worker_addr = %tcp_outlet.worker_addr, "Restoring outlet");
            let _ = node_manager
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
}
