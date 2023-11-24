use crate::state::AppState;
use crate::Error;
use tracing::{debug, info};

impl AppState {
    /// Delete a Local Service and relative TCP outlet from the default node.
    pub async fn delete_local_service(&self, service_name: String) -> crate::Result<()> {
        debug!(%service_name, "Deleting a local service");
        self.model_mut(|m| m.delete_local_service(&service_name))
            .await?;
        self.publish_state().await;

        let node_manager = self.node_manager().await;
        match node_manager.delete_outlet(&service_name).await {
            Ok(_) => {
                info!(%service_name, "TCP outlet deleted");
                Ok(())
            }
            Err(_) => Err(Error::App("Failed to delete TCP outlet".to_string())),
        }
    }
}
