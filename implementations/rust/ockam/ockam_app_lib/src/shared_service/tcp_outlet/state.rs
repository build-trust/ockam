use std::sync::Arc;
use tracing::{debug, error};

#[cfg(test)]
use crate::incoming_services::PersistentIncomingService;
use crate::state::{AppState, ModelState};
use ockam::transport::HostnamePort;
use ockam::Address;
use ockam_api::nodes::models::portal::{OutletAccessControl, OutletStatus};

impl ModelState {
    pub fn add_tcp_outlet(&mut self, status: OutletStatus) {
        self.tcp_outlets.push(status);
    }

    pub fn delete_tcp_outlet(&mut self, worker_addr: &Address) {
        self.tcp_outlets.retain(|x| &x.worker_addr != worker_addr);
    }

    pub fn get_tcp_outlets(&self) -> &[OutletStatus] {
        &self.tcp_outlets
    }

    #[cfg(test)]
    pub fn add_incoming_service(&mut self, service: PersistentIncomingService) {
        self.incoming_services.push(service);
    }
}

impl AppState {
    pub(crate) async fn restore_tcp_outlets(&self) {
        let cli_state = self.state().await;
        if !cli_state.is_enrolled().await.ok().unwrap_or(false) {
            debug!("Not enrolled, skipping outlet restoration");
            return;
        }
        let node_manager = self.node_manager().await;
        let context = self.context();
        for tcp_outlet in self.model(|m| m.get_tcp_outlets().to_vec()).await {
            let access_control = match self
                .create_invitations_access_control(tcp_outlet.worker_addr.clone())
                .await
            {
                Ok(a) => a,
                Err(e) => {
                    error!(
                        ?e,
                        worker_addr = %tcp_outlet.worker_addr,
                        "Failed to create access control"
                    );
                    continue;
                }
            };

            let incoming_ac = access_control.create_incoming();
            let outgoing_ac = match access_control.create_outgoing(self.context_ref()).await {
                Ok(a) => a,
                Err(e) => {
                    error!(
                        ?e,
                        worker_addr = %tcp_outlet.worker_addr,
                        "Failed to create access control"
                    );
                    continue;
                }
            };

            debug!(worker_addr = %tcp_outlet.worker_addr, "Restoring outlet");
            let _ = node_manager
                .create_outlet(
                    &context,
                    HostnamePort::from_socket_addr(tcp_outlet.socket_addr)
                        .expect("cannot parse the socket address as a hostname and port"),
                    false,
                    Some(tcp_outlet.worker_addr.clone()),
                    true,
                    OutletAccessControl::AccessControl((
                        Arc::new(incoming_ac),
                        Arc::new(outgoing_ac),
                    )),
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
