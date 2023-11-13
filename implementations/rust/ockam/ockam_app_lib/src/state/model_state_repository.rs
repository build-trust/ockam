use ockam_core::async_trait;

use crate::state::model::ModelState;
use crate::Result;

/// The ModelStateRepository is responsible for storing and loading
/// the persistent data managed by the desktop application.
#[async_trait]
pub trait ModelStateRepository: Send + Sync + 'static {
    /// Store / update the full model state in the database
    async fn store(&self, model_state: &ModelState) -> Result<()>;

    /// Load the model state from the database
    async fn load(&self) -> Result<ModelState>;
}
