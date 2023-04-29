use ockam::identity::{
    AttributesEntry, IdentitiesReader, IdentitiesRepository, IdentitiesWriter, Identity,
    IdentityAttributesReader, IdentityAttributesWriter, IdentityIdentifier, Timestamp,
};
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{collections::HashMap, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::path::PathBuf;
use tracing::trace;

#[derive(Clone)]
pub struct BootstrapedIdentityStore {
    bootstrapped: Arc<dyn IdentityAttributesReader>,
    repository: Arc<dyn IdentitiesRepository>,
}

impl BootstrapedIdentityStore {
    pub fn new(
        bootstrapped: Arc<dyn IdentityAttributesReader>,
        repository: Arc<dyn IdentitiesRepository>,
    ) -> Self {
        Self {
            bootstrapped,
            repository,
        }
    }
}

#[async_trait]
impl IdentityAttributesReader for BootstrapedIdentityStore {
    async fn get_attributes(
        &self,
        identity_id: &IdentityIdentifier,
    ) -> Result<Option<AttributesEntry>> {
        trace! {
            target: "ockam_api::bootstrapped_identities_store",
            id     = %identity_id,
            "get_attributes"
        }
        match self.bootstrapped.get_attributes(identity_id).await? {
            None => self.repository.get_attributes(identity_id).await,
            Some(x) => Ok(Some(x)),
        }
    }

    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>> {
        let mut l = self.repository.list().await?;
        let mut l2 = self.bootstrapped.list().await?;
        l.append(&mut l2);
        Ok(l)
    }
}

#[async_trait]
impl IdentityAttributesWriter for BootstrapedIdentityStore {
    async fn put_attributes(
        &self,
        sender: &IdentityIdentifier,
        entry: AttributesEntry,
    ) -> Result<()> {
        trace! {
            target: "ockam_api::bootstrapped_identities_store",
            id     = %sender,
            "put_attributes"
        }
        match self.bootstrapped.get_attributes(sender).await? {
            None => self.repository.put_attributes(sender, entry).await,
            Some(_) => Ok(()),
            /*
                 * TODO: previous client automatically adds the project admin as a member.
                 *       that is not needed, as the admin is part of the trusted anchors for the
                 *       authority already.
                 *       Remove the Ok() clause and replace by this error once we don't need to
                 *       support the old cli version anymore.
                    Err(ockam_core::Error::new(
                Origin::Identity,
                Kind::AlreadyExists,
                "cant write attributes for a bootstrapped identity",
            )),
                */
        }
    }

    async fn put_attribute_value(
        &self,
        subject: &IdentityIdentifier,
        attribute_name: &str,
        attribute_value: &str,
    ) -> Result<()> {
        self.repository
            .put_attribute_value(subject, attribute_name, attribute_value)
            .await
    }

    async fn delete(&self, identity: &IdentityIdentifier) -> Result<()> {
        self.repository.delete(identity).await
    }
}

#[async_trait]
impl IdentitiesReader for BootstrapedIdentityStore {
    async fn get_identity(&self, identifier: &IdentityIdentifier) -> Result<Option<Identity>> {
        self.repository.get_identity(identifier).await
    }
}

#[async_trait]
impl IdentitiesWriter for BootstrapedIdentityStore {
    async fn update_known_identity(&self, identity: &Identity) -> Result<()> {
        self.repository.update_known_identity(identity).await
    }
}

impl IdentitiesRepository for BootstrapedIdentityStore {
    fn as_attributes_reader(&self) -> Arc<dyn IdentityAttributesReader> {
        Arc::new(self.clone())
    }

    fn as_attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter> {
        Arc::new(self.clone())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PreTrustedIdentities {
    Fixed(HashMap<IdentityIdentifier, AttributesEntry>),
    ReloadFrom(PathBuf),
}

impl PreTrustedIdentities {
    pub fn new_from_disk(path: PathBuf, reload: bool) -> Result<Self> {
        if reload {
            Ok(PreTrustedIdentities::ReloadFrom(path))
        } else {
            Ok(PreTrustedIdentities::Fixed(Self::parse_from_disk(&path)?))
        }
    }

    pub fn new_from_string(entries: &str) -> Result<Self> {
        Ok(Self::new_from_hashmap(Self::parse(entries)?))
    }

    pub fn new_from_hashmap(entries: HashMap<IdentityIdentifier, AttributesEntry>) -> Self {
        PreTrustedIdentities::Fixed(entries)
    }

    pub fn get_trusted_identities(self) -> Result<HashMap<IdentityIdentifier, AttributesEntry>> {
        match self {
            PreTrustedIdentities::Fixed(identities) => Ok(identities),
            PreTrustedIdentities::ReloadFrom(path) => Self::parse_from_disk(&path),
        }
    }

    fn parse_from_disk(path: &PathBuf) -> Result<HashMap<IdentityIdentifier, AttributesEntry>> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Io, e))?;
        Self::parse(&contents)
    }

    fn parse(entries: &str) -> Result<HashMap<IdentityIdentifier, AttributesEntry>> {
        let raw_map =
            json::from_str::<HashMap<IdentityIdentifier, HashMap<String, String>>>(entries)
                .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Invalid, e))?;
        Ok(raw_map
            .into_iter()
            .map(|(identity_id, raw_attrs)| {
                let attrs = raw_attrs
                    .into_iter()
                    .map(|(k, v)| (k, v.as_bytes().to_vec()))
                    .collect();
                (
                    identity_id,
                    AttributesEntry::new(attrs, Timestamp::now().unwrap(), None, None),
                )
            })
            .collect())
    }
}

impl From<HashMap<IdentityIdentifier, AttributesEntry>> for PreTrustedIdentities {
    fn from(h: HashMap<IdentityIdentifier, AttributesEntry>) -> PreTrustedIdentities {
        PreTrustedIdentities::Fixed(h)
    }
}

#[async_trait]
impl IdentityAttributesReader for PreTrustedIdentities {
    async fn get_attributes(
        &self,
        identity_id: &IdentityIdentifier,
    ) -> Result<Option<AttributesEntry>> {
        match self {
            PreTrustedIdentities::Fixed(trusted) => Ok(trusted.get(identity_id).cloned()),
            PreTrustedIdentities::ReloadFrom(path) => {
                Ok(Self::parse_from_disk(path)?.get(identity_id).cloned())
            }
        }
    }

    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>> {
        match self {
            PreTrustedIdentities::Fixed(trusted) => Ok(trusted
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect()),
            PreTrustedIdentities::ReloadFrom(path) => Ok(Self::parse_from_disk(path)?
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect()),
        }
    }
}
