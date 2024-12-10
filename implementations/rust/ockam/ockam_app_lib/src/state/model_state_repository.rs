use ockam_core::async_trait;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

use crate::state::model::ModelState;
use crate::Result;

/// The ModelStateRepository is responsible for storing and loading
/// the persistent data managed by the desktop application.
#[async_trait]
pub trait ModelStateRepository: Send + Sync + 'static {
    /// Store / update the full model state in the database
    async fn store(&self, node_name: &str, model_state: &ModelState) -> Result<()>;

    /// Load the model state from the database
    async fn load(&self, node_name: &str) -> Result<ModelState>;
}

#[async_trait]
impl<T: ModelStateRepository> ModelStateRepository for AutoRetry<T> {
    async fn store(&self, node_name: &str, model_state: &ModelState) -> Result<()> {
        retry!(self.wrapped.store(node_name, model_state))
    }

    async fn load(&self, node_name: &str) -> Result<ModelState> {
        retry!(self.wrapped.load(node_name))
    }
}
