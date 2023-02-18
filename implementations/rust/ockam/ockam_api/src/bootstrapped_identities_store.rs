use ockam_core::async_trait;
use ockam_core::compat::{collections::HashMap, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{AsyncTryClone, Result};
use ockam_identity::authenticated_storage::{
    AttributesEntry, IdentityAttributeStorage, IdentityAttributeStorageReader,
    IdentityAttributeStorageWriter,
};
use ockam_identity::credential::Timestamp;
use ockam_identity::IdentityIdentifier;
use serde_json as json;
use std::path::PathBuf;
use tracing::trace;

#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
#[derive(Debug)]
pub struct BootstrapedIdentityStore<B: IdentityAttributeStorageReader, S: IdentityAttributeStorage>
{
    bootstrapped: B,
    storage: S,
}

impl<B: IdentityAttributeStorageReader, S: IdentityAttributeStorage>
    BootstrapedIdentityStore<B, S>
{
    pub fn new(bootstrapped: B, storage: S) -> Self {
        Self {
            bootstrapped,
            storage,
        }
    }
}

#[async_trait]
impl<B: IdentityAttributeStorageReader, S: IdentityAttributeStorage> IdentityAttributeStorageReader
    for BootstrapedIdentityStore<B, S>
{
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
            None => self.storage.get_attributes(identity_id).await,
            Some(x) => Ok(Some(x)),
        }
    }

    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>> {
        let mut l = self.storage.list().await?;
        let mut l2 = self.bootstrapped.list().await?;
        l.append(&mut l2);
        Ok(l)
    }
}

#[async_trait]
impl<B: IdentityAttributeStorageReader, S: IdentityAttributeStorage> IdentityAttributeStorageWriter
    for BootstrapedIdentityStore<B, S>
{
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
            None => self.storage.put_attributes(sender, entry).await,
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
}

impl<B: IdentityAttributeStorageReader, S: IdentityAttributeStorage> IdentityAttributeStorage
    for BootstrapedIdentityStore<B, S>
{
}

#[derive(Clone, Debug)]
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
        Ok(PreTrustedIdentities::Fixed(Self::parse(entries)?))
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
impl IdentityAttributeStorageReader for PreTrustedIdentities {
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
