use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

use crate::identity::IdentityConstants;
use crate::models::{ChangeHistory, Identifier};
use crate::storage::{InMemoryStorage, Storage};
use crate::utils::now;
use crate::{
    AttributesEntry, IdentitiesReader, IdentitiesRepository, IdentitiesWriter,
    IdentityAttributesReader, IdentityAttributesWriter,
};

/// Implementation of `IdentityAttributes` trait based on an underlying `Storage`
#[derive(Clone)]
pub struct IdentitiesStorage {
    storage: Arc<dyn Storage>,
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

impl IdentitiesStorage {
    /// Create a new storage for attributes
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Create a new storage for attributes
    pub fn create() -> Arc<Self> {
        Arc::new(Self::new(InMemoryStorage::create()))
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
