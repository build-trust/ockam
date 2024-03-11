use crate::models::CredentialAndPurposeKey;
use crate::{CredentialRetriever, CredentialsCache, Identifier};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

/// This retriever retrieves credentials from local storage
pub struct CachedCredentialRetriever {
    issuer: Identifier,
    credentials_cache: Arc<CredentialsCache>,
}

impl CachedCredentialRetriever {
    /// Create a new cached credential retriever
    pub fn new(
        issuer: &Identifier,
        credentials_cache: Arc<CredentialsCache>,
    ) -> CachedCredentialRetriever {
        Self {
            issuer: issuer.clone(),
            credentials_cache,
        }
    }
}

#[async_trait]
impl CredentialRetriever for CachedCredentialRetriever {
    async fn retrieve(&self, subject: &Identifier) -> Result<CredentialAndPurposeKey> {
        self.credentials_cache
            .get_credential(&self.issuer, subject)
            .await
    }
}
