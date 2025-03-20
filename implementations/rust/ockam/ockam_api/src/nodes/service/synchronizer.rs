use crate::nodes::NodeManager;
use ockam_node::Context;
use std::sync::{Arc, Weak};
use std::time::Duration;

/// Synchronizes the state of the portals with the database
pub struct PortalSynchronizer {
    node_manager: Weak<NodeManager>,
    pub context: Arc<Context>,
    pub interval: Duration,
}

impl PortalSynchronizer {
    pub fn new(context: Arc<Context>, interval: Duration, node_manager: Weak<NodeManager>) -> Self {
        Self {
            context,
            interval,
            node_manager,
        }
    }

    /// Start synchronization loop in a separate task
    pub async fn start(self) {
        tokio::spawn(async move {
            loop {
                let result = self.synchronize_state().await;
                if let Err(error) = result {
                    error!("Failed to synchronize state: {error:?}");
                }
                tokio::time::sleep(self.interval).await;
            }
        });
    }

    /// Synchronize state by fetching the list of and inlets from the database and
    /// comparing it with the list of outlets in the node manager
    async fn synchronize_state(&self) -> ockam_core::Result<()> {
        if let Some(node_manager) = self.node_manager.upgrade() {
            self.synchronize_tcp_outlets(node_manager).await?;
        }
        Ok(())
    }

    async fn synchronize_tcp_outlets(
        &self,
        node_manager: Arc<NodeManager>,
    ) -> ockam_core::Result<()> {
        let database_outlets = node_manager
            .cli_state
            .list_tcp_outlets(&node_manager.node_name)
            .await?;

        let registry_outlets = node_manager.list_outlets();

        // delete outlets that are not in the database, or they are different from the database
        let to_delete = registry_outlets
            .iter()
            .filter(|registry_outlet| {
                database_outlets
                    .iter()
                    .find(|database_outlet| {
                        database_outlet.worker_address == registry_outlet.worker_address
                    })
                    .filter(|database_outlet| database_outlet != registry_outlet)
                    .is_none()
            })
            .collect::<Vec<_>>();

        // create outlets that are in the database, but not in the registry, or they are different from the registry
        let to_create = database_outlets
            .iter()
            .filter(|database_outlet| {
                registry_outlets
                    .iter()
                    .find(|registry_outlet| {
                        registry_outlet.worker_address == database_outlet.worker_address
                    })
                    .filter(|registry_outlet| registry_outlet != database_outlet)
                    .is_none()
            })
            .collect::<Vec<_>>();

        for outlet in to_create {
            let result = node_manager
                .create_outlet(&self.context, outlet.parameters.clone())
                .await;
            if let Err(error) = result {
                error!("Failed to create outlet: {error:?}");
            }
        }

        for outlet in to_delete {
            let result = node_manager.delete_outlet(&outlet.worker_address).await;
            if let Err(error) = result {
                error!("Failed to delete outlet: {error:?}");
            }
        }

        Ok(())
    }
}
