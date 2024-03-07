use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{
    CredentialRepository, CredentialRetriever, CredentialRetrieverCreator, Identifier,
    IdentityError, TimestampInSeconds,
};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Result};
use tracing::{debug, error};

/// Credential is considered already expired if it expires in less than this gap to account for a machine with a
/// wrong time
pub const DEFAULT_CREDENTIAL_CLOCK_SKEW_GAP: TimestampInSeconds = TimestampInSeconds(60);

/// Credentials retriever for credentials located in a local cache
pub struct CachedCredentialRetriever {
    issuer: Identifier,
    subject: Identifier,
    scope: String,
    cache: Arc<dyn CredentialRepository>,
}

impl CachedCredentialRetriever {
    /// Create a new cache credential retriever
    pub fn new(
        issuer: Identifier,
        subject: Identifier,
        scope: String,
        cache: Arc<dyn CredentialRepository>,
    ) -> Self {
        Self {
            issuer,
            subject,
            scope,
            cache,
        }
    }

    /// Retrieve a credential from the credentials storage and check its expiration
    pub async fn retrieve_impl(
        issuer: &Identifier,
        for_identity: &Identifier,
        scope: &str,
        now: TimestampInSeconds,
        cache: Arc<dyn CredentialRepository>,
        clock_skew_gap: TimestampInSeconds,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        debug!(
            "Requested credential for: {} from: {}",
            for_identity, issuer
        );

        // check if we have a valid cached credential
        if let Some(cached_credential) = cache.get(for_identity, issuer, scope).await? {
            // add an extra minute to have a bit of leeway for clock skew
            if cached_credential.get_expires_at()? > now + clock_skew_gap {
                debug!("Found valid cached credential for: {}", for_identity);
                Ok(Some(cached_credential))
            } else {
                debug!(
                    "Found expired cached credential for: {}. Deleting...",
                    for_identity
                );
                let delete_res = cache.delete(for_identity, issuer, scope).await;

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

/// Creator for [`CachedCredentialRetriever`]
pub struct CachedCredentialRetrieverCreator {
    issuer: Identifier,
    scope: String,
    cache: Arc<dyn CredentialRepository>,
}

impl CachedCredentialRetrieverCreator {
    /// Constructor
    pub fn new(issuer: Identifier, scope: String, cache: Arc<dyn CredentialRepository>) -> Self {
        Self {
            issuer,
            scope,
            cache,
        }
    }
}

#[async_trait]
impl CredentialRetrieverCreator for CachedCredentialRetrieverCreator {
    async fn create(&self, subject: &Identifier) -> Result<Arc<dyn CredentialRetriever>> {
        Ok(Arc::new(CachedCredentialRetriever::new(
            self.issuer.clone(),
            subject.clone(),
            self.scope.clone(),
            self.cache.clone(),
        )))
    }
}

#[async_trait]
impl CredentialRetriever for CachedCredentialRetriever {
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn retrieve(&self) -> Result<CredentialAndPurposeKey> {
        let now = now()?;
        match Self::retrieve_impl(
            &self.issuer,
            &self.subject,
            &self.scope,
            now,
            self.cache.clone(),
            // We can't refresh the credential, so let's still present it even if it's
            // potentially expired
            0.into(),
        )
        .await?
        {
            Some(credential) => Ok(credential),
            None => Err(IdentityError::NoCredential)?,
        }
    }

    fn subscribe(&self, _address: &Address) -> Result<()> {
        Ok(())
    }

    fn unsubscribe(&self, _address: &Address) -> Result<()> {
        Ok(())
    }
}
