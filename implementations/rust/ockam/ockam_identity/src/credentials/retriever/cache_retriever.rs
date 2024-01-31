use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{CredentialRepository, CredentialRetriever, Identifier, TimestampInSeconds};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;
use tracing::{debug, error};

/// Credentials retriever for credentials located in a local cache
pub struct CachedCredentialRetriever {
    issuer: Identifier,
    cache: Arc<dyn CredentialRepository>,
}

impl CachedCredentialRetriever {
    /// Create a new cache credential retriever
    pub fn new(issuer: Identifier, cache: Arc<dyn CredentialRepository>) -> Self {
        Self { issuer, cache }
    }

    /// Retrieve a credential from the credentials storage and check its expiration
    pub async fn retrieve_impl(
        issuer: &Identifier,
        for_identity: &Identifier,
        cache: Arc<dyn CredentialRepository>,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        debug!(
            "Requested credential for: {} from: {}",
            for_identity, issuer
        );

        // check if we have a valid cached credential
        let now = now()?;
        if let Some(cached_credential) = cache.get(for_identity, issuer).await? {
            // add an extra minute to have a bit of leeway for clock skew
            if cached_credential.get_credential_data()?.expires_at > now + TimestampInSeconds(60) {
                debug!("Found valid cached credential for: {}", for_identity);
                Ok(Some(cached_credential))
            } else {
                debug!(
                    "Found expired cached credential for: {}. Deleting...",
                    for_identity
                );
                let delete_res = cache.delete(for_identity, issuer).await;

                if let Some(err) = delete_res.err() {
                    error!(
                        "Error deleting expired credential for {} from {}. Err={}",
                        for_identity, issuer, err
                    );
                }
                Ok(None)
            }
        } else {
            debug!("Found no cached credential for: {}", for_identity);
            Ok(None)
        }
    }
}

#[async_trait]
impl CredentialRetriever for CachedCredentialRetriever {
    async fn retrieve(
        &self,
        _ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        Self::retrieve_impl(&self.issuer, for_identity, self.cache.clone()).await
    }
}
