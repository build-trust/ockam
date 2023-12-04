use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json as json;
use tracing::trace;

use ockam::identity::utils::now;
use ockam::identity::{AttributesEntry, Identifier, IdentityAttributesRepository};
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{collections::HashMap, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;

#[derive(Clone)]
pub struct BootstrapedIdentityAttributesStore {
    bootstrapped: Arc<dyn IdentityAttributesRepository>,
    repository: Arc<dyn IdentityAttributesRepository>,
}

impl BootstrapedIdentityAttributesStore {
    pub fn new(
        bootstrapped: Arc<dyn IdentityAttributesRepository>,
        repository: Arc<dyn IdentityAttributesRepository>,
    ) -> Self {
        Self {
            bootstrapped,
            repository,
        }
    }
}

#[async_trait]
impl IdentityAttributesRepository for BootstrapedIdentityAttributesStore {
    async fn get_attributes(&self, identity_id: &Identifier) -> Result<Option<AttributesEntry>> {
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

    async fn list_attributes_by_identifier(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
        let mut l = self.repository.list_attributes_by_identifier().await?;
        let mut l2 = self.bootstrapped.list_attributes_by_identifier().await?;
        l.append(&mut l2);
        Ok(l)
    }

    async fn put_attributes(&self, sender: &Identifier, entry: AttributesEntry) -> Result<()> {
        trace! {
            target: "ockam_api::bootstrapped_identities_store",
            id     = %sender,
            "put_attributes"
        }
        match self.bootstrapped.get_attributes(sender).await? {
            None => self.repository.put_attributes(sender, entry).await,
            // FIXME: allow overwriting the attributes?
            Some(_) => Err(ockam_core::Error::new(
                Origin::Identity,
                Kind::AlreadyExists,
                "cant write attributes for a bootstrapped identity",
            )),
        }
    }

    async fn delete(&self, identity: &Identifier) -> Result<()> {
        self.repository.delete(identity).await
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PreTrustedIdentities {
    Fixed(HashMap<Identifier, AttributesEntry>),
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

    pub fn new_from_hashmap(entries: HashMap<Identifier, AttributesEntry>) -> Self {
        PreTrustedIdentities::Fixed(entries)
    }

    fn parse_from_disk(path: &PathBuf) -> Result<HashMap<Identifier, AttributesEntry>> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Io, e))?;
        Self::parse(&contents)
    }

    fn parse(entries: &str) -> Result<HashMap<Identifier, AttributesEntry>> {
        let raw_map = json::from_str::<HashMap<Identifier, HashMap<String, String>>>(entries)
            .map_err(|e| ockam_core::Error::new(Origin::Other, Kind::Invalid, e))?;
        let now = now()?;
        Ok(raw_map
            .into_iter()
            .map(|(identity_id, raw_attrs)| {
                let attrs = raw_attrs
                    .into_iter()
                    .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
                    .collect();
                (identity_id, AttributesEntry::new(attrs, now, None, None))
            })
            .collect())
    }
}

impl From<HashMap<Identifier, AttributesEntry>> for PreTrustedIdentities {
    fn from(h: HashMap<Identifier, AttributesEntry>) -> PreTrustedIdentities {
        PreTrustedIdentities::Fixed(h)
    }
}

#[async_trait]
impl IdentityAttributesRepository for PreTrustedIdentities {
    async fn get_attributes(&self, identity_id: &Identifier) -> Result<Option<AttributesEntry>> {
        match self {
            PreTrustedIdentities::Fixed(trusted) => Ok(trusted.get(identity_id).cloned()),
            PreTrustedIdentities::ReloadFrom(path) => {
                Ok(Self::parse_from_disk(path)?.get(identity_id).cloned())
            }
        }
    }

    async fn list_attributes_by_identifier(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
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

    async fn put_attributes(&self, _identity: &Identifier, _entry: AttributesEntry) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, _identity: &Identifier) -> Result<()> {
        Ok(())
    }
}
