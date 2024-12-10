use crate::models::CredentialAndPurposeKey;
use crate::{Identifier, TimestampInSeconds};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;

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

#[cfg(feature = "std")]
#[async_trait]
impl<T: CredentialRepository> CredentialRepository for AutoRetry<T> {
    async fn get(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        retry!(self.wrapped.get(subject, issuer, scope))
    }

    async fn put(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
        expires_at: TimestampInSeconds,
        credential: CredentialAndPurposeKey,
    ) -> Result<()> {
        retry!(self
            .wrapped
            .put(subject, issuer, scope, expires_at, credential.clone()))
    }

    async fn delete(&self, subject: &Identifier, issuer: &Identifier, scope: &str) -> Result<()> {
        retry!(self.wrapped.delete(subject, issuer, scope))
    }
}
