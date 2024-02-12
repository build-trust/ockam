use crate::state::AppState;
use crate::Error;
use ockam_core::Address;
use tracing::{debug, info};

impl AppState {
    /// Delete a TCP outlet from the default node.
    pub async fn tcp_outlet_delete(&self, worker_addr: Address) -> crate::Result<()> {
        debug!(%worker_addr, "Deleting a TCP outlet");
        let node_manager = self.node_manager().await;
        match node_manager.delete_outlet(&worker_addr).await {
            Ok(_) => {
                info!(%worker_addr, "TCP outlet deleted");
                self.model_mut(|m| m.delete_tcp_outlet(&worker_addr))
                    .await?;
                self.publish_state().await;
                Ok(())
            }
            Err(_) => Err(Error::App("Failed to delete TCP outlet".to_string())),
        }?;
        Ok(())
    }
}
