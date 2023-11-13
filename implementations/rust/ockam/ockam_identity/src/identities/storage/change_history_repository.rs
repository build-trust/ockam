use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

use crate::models::{ChangeHistory, Identifier};

/// This repository stores identity change histories
#[async_trait]
pub trait ChangeHistoryRepository: Send + Sync + 'static {
    /// Store an identifier with its change history
    async fn store_change_history(
        &self,
        identifier: &Identifier,
        change_history: ChangeHistory,
    ) -> Result<()>;

    /// Delete a change history given its identifier
    async fn delete_change_history(&self, identifier: &Identifier) -> Result<()>;

    /// Return the change history of a persisted identity
    async fn get_change_history(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>>;

    /// Return all the change histories
    async fn get_change_histories(&self) -> Result<Vec<ChangeHistory>>;
}
