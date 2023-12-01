use crate::{AttributeName, AttributeValue, AttributesEntry, Identifier};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

/// This trait supports the persistence of attributes associated to identities
#[async_trait]
pub trait IdentityAttributesRepository: Send + Sync + 'static {
    /// Get the attributes associated with the given identity identifier
    async fn get_attributes(&self, subject: &Identifier) -> Result<Option<AttributesEntry>>;

    /// List all identities with their attributes
    async fn list_attributes_by_identifier(&self) -> Result<Vec<(Identifier, AttributesEntry)>>;

    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()>;

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: AttributeName,
        attribute_value: AttributeValue,
    ) -> Result<()>;

    /// Remove all attributes for a given identity identifier
    async fn delete(&self, identity: &Identifier) -> Result<()>;
}
