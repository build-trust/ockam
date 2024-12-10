use crate::models::{ChangeHistory, Identifier};
use crate::Identity;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;

/// This repository stores identity change histories
#[async_trait]
pub trait ChangeHistoryRepository: Send + Sync + 'static {
    /// Update the change history of an identity atomically
    ///  - verify that the new change history is compatible with the previous one
    ///  - store the new change history
    async fn update_identity(&self, identity: &Identity, ignore_older: bool) -> Result<()>;

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

#[cfg(feature = "std")]
#[async_trait]
impl<T: ChangeHistoryRepository> ChangeHistoryRepository for AutoRetry<T> {
    async fn update_identity(&self, identity: &Identity, ignore_older: bool) -> Result<()> {
        retry!(self.wrapped.update_identity(identity, ignore_older))
    }

    async fn store_change_history(
        &self,
        identifier: &Identifier,
        change_history: ChangeHistory,
    ) -> Result<()> {
        retry!(self
            .wrapped
            .store_change_history(identifier, change_history.clone()))
    }

    async fn delete_change_history(&self, identifier: &Identifier) -> Result<()> {
        retry!(self.wrapped.delete_change_history(identifier))
    }

    async fn get_change_history(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>> {
        retry!(self.wrapped.get_change_history(identifier))
    }

    async fn get_change_histories(&self) -> Result<Vec<ChangeHistory>> {
        retry!(self.wrapped.get_change_histories())
    }
}
