use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Result};

use crate::models::CredentialAndPurposeKey;
use crate::Identifier;

/// Trait for retrieving a credential for a given identity
#[async_trait]
pub trait CredentialRetriever: Send + Sync + 'static {
    /// Retrieve a credential for an identity.
    async fn retrieve(&self, subject: &Identifier) -> Result<CredentialAndPurposeKey>;
}
