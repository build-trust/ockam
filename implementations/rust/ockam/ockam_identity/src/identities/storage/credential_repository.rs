use crate::models::CredentialAndPurposeKey;
use crate::{Identifier, TimestampInSeconds};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

/// This trait supports the persistence of cached credentials
#[async_trait]
pub trait CredentialRepository: Send + Sync + 'static {
    /// Get credential
    async fn get(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
    ) -> Result<Option<CredentialAndPurposeKey>>;

    /// Put credential (overwriting)
    async fn put(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
        expires_at: TimestampInSeconds,
        credential: CredentialAndPurposeKey,
    ) -> Result<()>;

    /// Delete credential
    async fn delete(&self, subject: &Identifier, issuer: &Identifier, scope: &str) -> Result<()>;
}
