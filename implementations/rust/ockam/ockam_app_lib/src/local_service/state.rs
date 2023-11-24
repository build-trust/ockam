use tracing::{debug, error};

#[cfg(test)]
use crate::incoming_services::PersistentIncomingService;
use crate::state::{AppState, ModelState};
use ockam_core::Address;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PersistentLocalService {
    pub socket_addr: SocketAddr,
    pub worker_addr: Address,
    pub alias: String,
    pub scheme: Option<String>,
}

impl ModelState {
    pub fn add_local_service(&mut self, status: PersistentLocalService) {
        self.local_services.push(status);
    }

    pub fn delete_local_service(&mut self, alias: &str) {
        self.local_services.retain(|x| x.alias != alias);
    }

    pub fn get_local_services(&self) -> &[PersistentLocalService] {
        &self.local_services
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
        for local_service in self.model(|m| m.get_local_services().to_vec()).await {
            let access_control = match self
                .create_invitations_access_control(local_service.worker_addr.address().to_string())
                .await
            {
                Ok(a) => a,
                Err(e) => {
                    error!(
                        ?e,
                        worker_addr = %local_service.worker_addr,
                        "Failed to create access control"
                    );
                    continue;
                }
            };

            debug!(worker_addr = %local_service.worker_addr, "Restoring outlet");
            let _ = node_manager
                .create_outlet(
                    &context,
                    local_service.socket_addr,
                    local_service.worker_addr.clone(),
                    Some(local_service.alias.clone()),
                    true,
                    Some(access_control),
                )
                .await
                .map_err(|e| {
                    error!(
                        ?e,
                        worker_addr = %local_service.worker_addr,
                        "Failed to restore outlet"
                    );
                });
        }
    }
}
