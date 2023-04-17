use crate::alloc::string::ToString;
use crate::credential::Timestamp;
use crate::identities::storage::storage::{InMemoryStorage, Storage};
use crate::identity::IdentityHistoryComparison;
use crate::identity::{Identity, IdentityChangeConstants, IdentityError, IdentityIdentifier};
use crate::AttributesEntry;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;

/// Repository for data related to identities: key changes and attributes
#[async_trait]
pub trait IdentitiesRepository:
IdentityAttributesReader + IdentityAttributesWriter + IdentitiesReader + IdentitiesWriter
{
    /// Restrict this repository as a reader for attributes
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader>;

    /// Restrict this repository as a writer for attributes
    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter>;
}

#[async_trait]
impl IdentitiesRepository for IdentitiesStorage {
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader> {
        Arc::new(self.clone())
    }

    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter> {
        Arc::new(self.clone())
    }
}

/// Trait implementing read access to attributes
#[async_trait]
pub trait IdentityAttributesReader: Send + Sync + 'static {
    /// Get the attributes associated with the given identity identifier
    async fn get_attributes(
        &self,
        identity: &IdentityIdentifier,
    ) -> Result<Option<AttributesEntry>>;

    /// List all identities with their attributes
    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>>;
}

/// Trait implementing write access to attributes
#[async_trait]
pub trait IdentityAttributesWriter: Send + Sync + 'static {
    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(
        &self,
        identity: &IdentityIdentifier,
        entry: AttributesEntry,
    ) -> Result<()>;

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &IdentityIdentifier,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<()>;

    /// Remove all attributes for a given identity identifier
    async fn delete(&self, identity: &IdentityIdentifier) -> Result<()>;
}

/// Trait implementing write access to identities
#[async_trait]
pub trait IdentitiesWriter: Send + Sync + 'static {
    /// Store changes if there are new key changes associated to that identity
    /// Return an error if the current change history conflicts with the persisted one
    async fn update_known_identity(&self, identity: &Identity) -> Result<()>;
}

/// Trait implementing read access to identiets
#[async_trait]
pub trait IdentitiesReader: Send + Sync + 'static {
    /// Return a persisted identity
    async fn get_identity(&self, identifier: &IdentityIdentifier) -> Result<Option<Identity>>;
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
    async fn put_identity(&self, identity: &Identity) -> Result<()> {
        self.storage
            .set(
                &identity.identifier().to_string(),
                IdentityChangeConstants::CHANGE_HISTORY_KEY.to_string(),
                identity.export()?,
            )
            .await
    }
}

#[async_trait]
impl IdentityAttributesReader for IdentitiesStorage {
    async fn get_attributes(
        &self,
        identity_id: &IdentityIdentifier,
    ) -> Result<Option<AttributesEntry>> {
        let id = identity_id.to_string();
        let entry = match self
            .storage
            .get(&id, IdentityChangeConstants::ATTRIBUTES_KEY)
            .await?
        {
            Some(e) => e,
            None => return Ok(None),
        };

        let entry: AttributesEntry = minicbor::decode(&entry)?;

        let now = Timestamp::now().ok_or_else(|| {
            ockam_core::Error::new(Origin::Core, Kind::Internal, "invalid system time")
        })?;
        match entry.expires() {
            Some(exp) if exp <= now => {
                self.storage
                    .del(&id, IdentityChangeConstants::ATTRIBUTES_KEY)
                    .await?;
                Ok(None)
            }
            _ => Ok(Some(entry)),
        }
    }

    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>> {
        let mut l = Vec::new();
        for id in self
            .storage
            .keys(IdentityChangeConstants::ATTRIBUTES_KEY)
            .await?
        {
            let identity_identifier = IdentityIdentifier::try_from(id)?;
            if let Some(attrs) = self.get_attributes(&identity_identifier).await? {
                l.push((identity_identifier, attrs))
            }
        }
        Ok(l)
    }
}

#[async_trait]
impl IdentityAttributesWriter for IdentitiesStorage {
    async fn put_attributes(
        &self,
        sender: &IdentityIdentifier,
        entry: AttributesEntry,
    ) -> Result<()> {
        // TODO: Implement expiration mechanism in Storage
        let entry = minicbor::to_vec(&entry)?;

        self.storage
            .set(
                &sender.to_string(),
                IdentityChangeConstants::ATTRIBUTES_KEY.to_string(),
                entry,
            )
            .await?;

        Ok(())
    }

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &IdentityIdentifier,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<()> {
        let mut attributes = match self.get_attributes(subject).await? {
            Some(entry) => (*entry.attrs()).clone(),
            None => BTreeMap::new(),
        };
        attributes.insert(
            attribute_name.to_string(),
            attribute_value.as_bytes().to_vec(),
        );
        let entry = AttributesEntry::new(
            attributes,
            Timestamp::now().unwrap(),
            None,
            Some(subject.clone()),
        );
        self.put_attributes(subject, entry).await
    }

    async fn delete(&self, identity: &IdentityIdentifier) -> Result<()> {
        self.storage
            .del(
                identity.to_string().as_str(),
                IdentityChangeConstants::ATTRIBUTES_KEY,
            )
            .await
    }
}

#[async_trait]
impl IdentitiesWriter for IdentitiesStorage {
    async fn update_known_identity(&self, identity: &Identity) -> Result<()> {
        let should_set = if let Some(known) = self.get_identity(&identity.identifier()).await? {
            match identity.changes().compare(known.changes()) {
                IdentityHistoryComparison::Equal => false, /* Do nothing */
                IdentityHistoryComparison::Conflict => {
                    return Err(IdentityError::ConsistencyError.into());
                }
                IdentityHistoryComparison::Newer => true, /* Update */
                IdentityHistoryComparison::Older => {
                    return Err(IdentityError::ConsistencyError.into());
                }
            }
        } else {
            true
        };

        if should_set {
            self.put_identity(identity).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl IdentitiesReader for IdentitiesStorage {
    async fn get_identity(&self, identifier: &IdentityIdentifier) -> Result<Option<Identity>> {
        if let Some(data) = self
            .storage
            .get(
                &identifier.to_string(),
                IdentityChangeConstants::CHANGE_HISTORY_KEY,
            )
            .await?
        {
            Ok(Some(Identity::import(identifier, &data)?))
        } else {
            Ok(None)
        }
    }
}
