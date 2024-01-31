use crate::{AttributesEntry, Identifier, TimestampInSeconds};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

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
