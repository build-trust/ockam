use crate::state::AppState;
use crate::Error;
use tracing::{debug, info};

impl AppState {
    /// Delete a TCP outlet from the default node.
    pub async fn tcp_outlet_delete(&self, alias: String) -> crate::Result<()> {
        debug!(%alias, "Deleting a TCP outlet");
        let node_manager = self.node_manager().await;
        match node_manager.delete_outlet(&alias).await {
            Ok(_) => {
                info!(%alias, "TCP outlet deleted");
                self.model_mut(|m| m.delete_tcp_outlet(&alias)).await?;
                self.publish_state().await;
                Ok(())
            }
            Err(_) => Err(Error::App("Failed to delete TCP outlet".to_string())),
        }?;
        Ok(())
    }
}
