use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, Error};

use super::super::super::identities::storage::storage::{InMemoryStorage, Storage};
use super::super::super::identity::IdentityConstants;
use super::super::super::models::{ChangeHistory, Identifier};
use super::super::super::utils::now;
use super::super::AttributesEntry;
use ockam_core::compat::string::ToString;

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

#[async_trait]
impl IdentitiesRepository for IdentitiesStorage {
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader> {
        Arc::new(self.clone())
    }

    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter> {
        Arc::new(self.clone())
    }

    fn as_identities_reader(&self) -> Arc<dyn IdentitiesReader> {
        Arc::new(self.clone())
    }

    fn as_identities_writer(&self) -> Arc<dyn IdentitiesWriter> {
        Arc::new(self.clone())
    }
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
    /// Return an error if the current change history conflicts with the persisted one
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

/// Implementation of `IdentityAttributes` trait based on an underlying `Storage`
#[derive(Clone)]
pub struct IdentitiesStorage {
    storage: Arc<dyn Storage>,
}

impl Default for IdentitiesStorage {
    fn default() -> IdentitiesStorage {
        IdentitiesStorage {
            storage: Arc::new(InMemoryStorage::new()),
        }
    }
}

impl IdentitiesStorage {
    /// Create a new storage for attributes
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Create a new storage for attributes
    pub fn create() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Persist an Identity (overrides it)
    async fn put_identity(
        &self,
        identifier: &Identifier,
        change_history: &ChangeHistory,
    ) -> Result<()> {
        self.storage
            .set(
                &identifier.to_string(),
                IdentityConstants::CHANGE_HISTORY_KEY.to_string(),
                minicbor::to_vec(change_history)?,
            )
            .await
    }
}

#[async_trait]
impl IdentityAttributesReader for IdentitiesStorage {
    async fn get_attributes(&self, identity_id: &Identifier) -> Result<Option<AttributesEntry>> {
        let id = identity_id.to_string();
        let entry = match self
            .storage
            .get(&id, IdentityConstants::ATTRIBUTES_KEY)
            .await?
        {
            Some(e) => e,
            None => return Ok(None),
        };

        let entry: AttributesEntry = minicbor::decode(&entry)?;

        let now = now()?;
        match entry.expires() {
            Some(exp) if exp <= now => {
                self.storage
                    .del(&id, IdentityConstants::ATTRIBUTES_KEY)
                    .await?;
                Ok(None)
            }
            _ => Ok(Some(entry)),
        }
    }

    async fn list(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
        let mut l = Vec::new();
        for id in self.storage.keys(IdentityConstants::ATTRIBUTES_KEY).await? {
            let identity_identifier = Identifier::try_from(id)?;
            if let Some(attrs) = self.get_attributes(&identity_identifier).await? {
                l.push((identity_identifier, attrs))
            }
        }
        Ok(l)
    }
}

#[async_trait]
impl IdentityAttributesWriter for IdentitiesStorage {
    async fn put_attributes(&self, sender: &Identifier, entry: AttributesEntry) -> Result<()> {
        // TODO: Implement expiration mechanism in Storage
        let entry = minicbor::to_vec(&entry)?;

        self.storage
            .set(
                &sender.to_string(),
                IdentityConstants::ATTRIBUTES_KEY.to_string(),
                entry,
            )
            .await?;

        Ok(())
    }

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()> {
        let mut attributes = match self.get_attributes(subject).await? {
            Some(entry) => (*entry.attrs()).clone(),
            None => BTreeMap::new(),
        };
        attributes.insert(attribute_name, attribute_value);
        let entry = AttributesEntry::new(attributes, now()?, None, Some(subject.clone()));
        self.put_attributes(subject, entry).await
    }

    async fn delete(&self, identity: &Identifier) -> Result<()> {
        self.storage
            .del(
                identity.to_string().as_str(),
                IdentityConstants::ATTRIBUTES_KEY,
            )
            .await
    }
}

#[async_trait]
impl IdentitiesWriter for IdentitiesStorage {
    async fn update_identity(
        &self,
        identifier: &Identifier,
        change_history: &ChangeHistory,
    ) -> Result<()> {
        // FIXME
        let should_set = true;
        // let should_set = if let Some(known) = self.retrieve_identity(&identity.identifier()).await?
        // {
        //     match identity.changes().compare(known.changes()) {
        //         IdentityHistoryComparison::Equal => false, /* Do nothing */
        //         IdentityHistoryComparison::Conflict => {
        //             return Err(IdentityError::ConsistencyError.into());
        //         }
        //         IdentityHistoryComparison::Newer => true, /* Update */
        //         IdentityHistoryComparison::Older => {
        //             return Err(IdentityError::ConsistencyError.into());
        //         }
        //     }
        // } else {
        //     true
        // };

        if should_set {
            self.put_identity(identifier, change_history).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl IdentitiesReader for IdentitiesStorage {
    async fn retrieve_identity(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>> {
        if let Some(data) = self
            .storage
            .get(
                &identifier.to_string(),
                IdentityConstants::CHANGE_HISTORY_KEY,
            )
            .await?
        {
            Ok(Some(minicbor::decode(&data)?))
        } else {
            Ok(None)
        }
    }
}
