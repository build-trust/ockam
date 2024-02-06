use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{CredentialRepository, Identifier, IdentityError, TimestampInSeconds};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use tracing::{debug, error};

/// This cache
pub struct CredentialsCache {
    repository: Arc<dyn CredentialRepository>,
}

impl CredentialsCache {
    /// Create a new cache credential retriever
    pub fn new(cache: Arc<dyn CredentialRepository>) -> Self {
        Self { repository: cache }
    }

    /// Retrieve a credential from the credentials storage and check its expiration
    pub async fn get_credential(
        &self,
        issuer: &Identifier,
        subject: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        debug!("Requested credential for: {} from: {}", subject, issuer);

        // check if we have a valid cached credential
        if let Some(cached_credential) = self.repository.get(subject, issuer).await? {
            if cached_credential.get_expires_at()? > now()? {
                debug!("Found valid cached credential for: {}.", subject);
                Ok(cached_credential)
            } else {
                debug!(
                    "Found expired cached credential for: {}. Deleting...",
                    subject
                );
                let delete_res = self.repository.delete(subject, issuer).await;

                if let Some(err) = delete_res.err() {
                    error!(
                        "Error deleting expired credential for {} from {}. Err={}",
                        subject, issuer, err
                    );
                }
                Err(IdentityError::NoCredential)?
            }
        } else {
            debug!("Found no cached credential for: {}", subject);
            Err(IdentityError::NoCredential)?
        }
    }

    /// Store a newly retrieved credential locally
    pub async fn store(
        &self,
        issuer: &Identifier,
        subject: &Identifier,
        expires_at: TimestampInSeconds,
        credential_and_purpose_key: CredentialAndPurposeKey,
    ) -> Result<()> {
        self.repository
            .put(subject, issuer, expires_at, credential_and_purpose_key)
            .await
    }
}
