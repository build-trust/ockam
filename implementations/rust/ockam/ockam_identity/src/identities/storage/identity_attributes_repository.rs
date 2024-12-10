use crate::{AttributesEntry, Identifier, TimestampInSeconds};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;

/// This trait supports the persistence of attributes associated to identities
#[async_trait]
pub trait IdentityAttributesRepository: Send + Sync + 'static {
    /// Get the attributes associated with the given identity identifier
    async fn get_attributes(
        &self,
        subject: &Identifier,
        attested_by: &Identifier,
    ) -> Result<Option<AttributesEntry>>;

    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()>;

    /// Remove all expired attributes
    async fn delete_expired_attributes(&self, now: TimestampInSeconds) -> Result<()>;
}

#[cfg(feature = "std")]
#[async_trait]
impl<T: IdentityAttributesRepository> IdentityAttributesRepository for AutoRetry<T> {
    async fn get_attributes(
        &self,
        subject: &Identifier,
        attested_by: &Identifier,
    ) -> Result<Option<AttributesEntry>> {
        retry!(self.wrapped.get_attributes(subject, attested_by))
    }

    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()> {
        retry!(self.wrapped.put_attributes(subject, entry.clone()))
    }

    async fn delete_expired_attributes(&self, now: TimestampInSeconds) -> Result<()> {
        retry!(self.wrapped.delete_expired_attributes(now))
    }
}
