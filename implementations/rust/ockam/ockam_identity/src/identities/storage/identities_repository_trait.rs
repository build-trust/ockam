use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, Error};

use crate::models::{ChangeHistory, Identifier};
use crate::AttributesEntry;

/// Repository for data related to identities: key changes and attributes
#[async_trait]
pub trait IdentitiesRepository:
    IdentityAttributesReader + IdentityAttributesWriter + IdentitiesReader + IdentitiesWriter
{
    /// Restrict this repository as a reader for attributes
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader>;

    /// Restrict this repository as a writer for attributes
    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter>;

    /// Restrict this repository as a reader for identities
    fn as_identities_reader(&self) -> Arc<dyn IdentitiesReader>;

    /// Restrict this repository as a writer for identities
    fn as_identities_writer(&self) -> Arc<dyn IdentitiesWriter>;
}

/// Trait implementing read access to attributes
#[async_trait]
pub trait IdentityAttributesReader: Send + Sync + 'static {
    /// Get the attributes associated with the given identity identifier
    async fn get_attributes(&self, identity: &Identifier) -> Result<Option<AttributesEntry>>;

    /// List all identities with their attributes
    async fn list(&self) -> Result<Vec<(Identifier, AttributesEntry)>>;
}

/// Trait implementing write access to attributes
#[async_trait]
pub trait IdentityAttributesWriter: Send + Sync + 'static {
    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(&self, identity: &Identifier, entry: AttributesEntry) -> Result<()>;

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()>;

    /// Remove all attributes for a given identity identifier
    async fn delete(&self, identity: &Identifier) -> Result<()>;
}

/// Trait implementing write access to identities
#[async_trait]
pub trait IdentitiesWriter: Send + Sync + 'static {
    /// Store changes if there are new key changes associated to that identity
    async fn update_identity(
        &self,
        identifier: &Identifier,
        change_history: &ChangeHistory,
    ) -> Result<()>;
}

/// Trait implementing read access to identiets
#[async_trait]
pub trait IdentitiesReader: Send + Sync + 'static {
    /// Return a persisted identity
    async fn retrieve_identity(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>>;

    /// Return a persisted identity that is expected to be present and return and Error if this is not the case
    async fn get_identity(&self, identifier: &Identifier) -> Result<ChangeHistory> {
        match self.retrieve_identity(identifier).await? {
            Some(change_history) => Ok(change_history),
            None => Err(Error::new(
                Origin::Core,
                Kind::NotFound,
                format!("identity not found for identifier {}", identifier),
            )),
        }
    }
}
