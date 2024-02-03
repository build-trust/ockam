use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, Result};

use crate::models::CredentialAndPurposeKey;
use crate::Identifier;

/// Trait for retrieving a credential for a given identity
#[async_trait]
pub trait CredentialRetriever: Send + Sync + 'static {
    /// Initialization of the retriever. Might load initial state, or start scheduled refresh events.
    async fn initialize(&self) -> Result<()>;

    /// Retrieve a credential for an identity.
    async fn retrieve(&self) -> Result<CredentialAndPurposeKey>;

    /// Subscribe to credential refresh
    fn subscribe(&self, address: &Address) -> Result<()>;

    /// Unsubscribe from credential refresh
    fn unsubscribe(&self, address: &Address) -> Result<()>;
}

/// Creator for [`CredentialRetriever`] implementation
#[async_trait]
pub trait CredentialRetrieverCreator: Send + Sync + 'static {
    /// Retrieve a credential for an identity.
    async fn create(&self, subject: &Identifier) -> Result<Arc<dyn CredentialRetriever>>;
}
