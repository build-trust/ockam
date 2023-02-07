use crate::alloc::borrow::ToOwned;
use crate::alloc::string::ToString;
use crate::credential::Timestamp;
use crate::{IdentityIdentifier, IdentityStateConst};
use minicbor::{Decode, Encode};
use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{AsyncTryClone, Result};

/// Storage for Authenticated data
#[async_trait]
pub trait AuthenticatedStorage: AsyncTryClone + Send + Sync + 'static {
    /// Get entry
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set entry
    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()>;

    /// Delete entry
    async fn del(&self, id: &str, key: &str) -> Result<()>;

    /// List all keys of a given "type".  TODO: we shouldn't store different things on a single
    /// store.
    async fn keys(&self, namespace: &str) -> Result<Vec<String>>;
}

/// An entry on the AuthenticatedIdentities table.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry {
    #[b(1)] attrs: BTreeMap<String, Vec<u8>>,
    #[n(2)] added: Timestamp,
    #[n(3)] expires: Option<Timestamp>,
    #[n(4)] attested_by: Option<IdentityIdentifier>,
}

impl AttributesEntry {
    /// Constructor
    pub fn new(
        attrs: BTreeMap<String, Vec<u8>>,
        added: Timestamp,
        expires: Option<Timestamp>,
        attested_by: Option<IdentityIdentifier>,
    ) -> Self {
        Self {
            attrs,
            added,
            expires,
            attested_by,
        }
    }

    /// The entry attributes
    pub fn attrs(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.attrs
    }

    /// Expiration time for this entry
    pub fn expires(&self) -> Option<Timestamp> {
        self.expires
    }

    /// Date that the entry was added
    pub fn added(&self) -> Timestamp {
        self.added
    }
    /// Who attested this attributes for this identity identifier
    pub fn attested_by(&self) -> Option<IdentityIdentifier> {
        self.attested_by.to_owned()
    }
}

/// Trait implementing read access to an AuthenticatedIdentities table
#[async_trait]
pub trait IdentityAttributeStorageReader: AsyncTryClone + Send + Sync + 'static {
    /// Get the attributes associated with the given identity identifier
    async fn get_attributes(
        &self,
        identity: &IdentityIdentifier,
    ) -> Result<Option<AttributesEntry>>;

    /// List all identities with their attributes
    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>>;
}

/// Trait implementing write access to an AuthenticatedIdentities table
#[async_trait]
pub trait IdentityAttributeStorageWriter: AsyncTryClone + Send + Sync + 'static {
    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(
        &self,
        identity: &IdentityIdentifier,
        entry: AttributesEntry,
    ) -> Result<()>;
}

/// Trait implementing read/write access to an AuthenticatedIdentities table
#[async_trait]
pub trait IdentityAttributeStorage:
    IdentityAttributeStorageReader + IdentityAttributeStorageWriter
{
}

/// Implementation of `IdentityAttributeStorage` trait based on an underling
/// `AuthenticatedStorage` store.
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
#[derive(Debug)]
pub struct AuthenticatedAttributeStorage<S: AuthenticatedStorage> {
    storage: S,
}

impl<S: AuthenticatedStorage> AuthenticatedAttributeStorage<S> {
    /// Constructor. `AttributesEntry` entries are serialized and stored on the underling
    /// storage given.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: AuthenticatedStorage> IdentityAttributeStorage for AuthenticatedAttributeStorage<S> {}

#[async_trait]
impl<S: AuthenticatedStorage> IdentityAttributeStorageReader for AuthenticatedAttributeStorage<S> {
    async fn list(&self) -> Result<Vec<(IdentityIdentifier, AttributesEntry)>> {
        let mut l = Vec::new();
        for id in self
            .storage
            .keys(IdentityStateConst::ATTRIBUTES_KEY)
            .await?
        {
            let identity_identifier = IdentityIdentifier::try_from(id)?;
            if let Some(attrs) = self.get_attributes(&identity_identifier).await? {
                l.push((identity_identifier, attrs))
            }
        }
        Ok(l)
    }

    async fn get_attributes(
        &self,
        identity_id: &IdentityIdentifier,
    ) -> Result<Option<AttributesEntry>> {
        let id = identity_id.to_string();
        let entry = match self
            .storage
            .get(&id, IdentityStateConst::ATTRIBUTES_KEY)
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
                    .del(&id, IdentityStateConst::ATTRIBUTES_KEY)
                    .await?;
                Ok(None)
            }
            _ => Ok(Some(entry)),
        }
    }
}

#[async_trait]
impl<S: AuthenticatedStorage> IdentityAttributeStorageWriter for AuthenticatedAttributeStorage<S> {
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
                IdentityStateConst::ATTRIBUTES_KEY.to_string(),
                entry,
            )
            .await?;

        Ok(())
    }
}

/// In-memory impl
pub mod mem;
