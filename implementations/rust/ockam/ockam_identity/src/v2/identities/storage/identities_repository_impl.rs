use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

use super::super::super::identity::IdentityConstants;
use super::super::super::models::{ChangeHistory, Identifier};
use super::super::super::storage::{InMemoryStorage, Storage};
use super::super::super::utils::now;
use super::super::super::{Identity, IdentityError, IdentityHistoryComparison};
use super::super::IdentitiesVault;
use super::{
    AttributesEntry, IdentitiesReader, IdentitiesRepository, IdentitiesWriter,
    IdentityAttributesReader, IdentityAttributesWriter,
};

/// Implementation of `IdentityAttributes` trait based on an underlying `Storage`
#[derive(Clone)]
pub struct IdentitiesStorage {
    storage: Arc<dyn Storage>,
    vault: Arc<dyn IdentitiesVault>, // TODO: Reconsider Vault dependency
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
    pub fn new(storage: Arc<dyn Storage>, vault: Arc<dyn IdentitiesVault>) -> Self {
        Self { storage, vault }
    }

    /// Create a new storage for attributes
    pub fn create(vault: Arc<dyn IdentitiesVault>) -> Arc<Self> {
        Arc::new(Self::new(InMemoryStorage::create(), vault))
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

    async fn should_set(&self, identity: &Identity) -> Result<bool> {
        let known = if let Some(known) = self.retrieve_identity(identity.identifier()).await? {
            known
        } else {
            return Ok(true);
        };

        let known = Identity::import_from_change_history(known, self.vault.clone()).await?;

        let res = match identity.compare(&known) {
            IdentityHistoryComparison::Equal => false, /* Do nothing */
            IdentityHistoryComparison::Conflict => {
                return Err(IdentityError::ConsistencyError.into());
            }
            IdentityHistoryComparison::Newer => true, /* Update */
            IdentityHistoryComparison::Older => {
                return Err(IdentityError::ConsistencyError.into());
            }
        };

        Ok(res)
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
    async fn update_identity(&self, identity: &Identity) -> Result<()> {
        let should_set = self.should_set(identity).await?;

        if should_set {
            self.put_identity(identity.identifier(), identity.change_history())
                .await?;
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
