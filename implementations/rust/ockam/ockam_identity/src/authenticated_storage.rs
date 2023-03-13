use crate::alloc::borrow::ToOwned;
use crate::alloc::string::ToString;
use crate::credential::Timestamp;
use crate::{IdentityIdentifier, IdentityStateConst};
use minicbor::{Decode, Encode};
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;

/// Storage for Authenticated data
#[async_trait]
pub trait AuthenticatedStorage: Send + Sync + 'static {
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
    //TODO: since we are converting from HashMap to BTreeMap in different parts,
    //      it will make sense to have a constructor here taking a HashMap and doing
    //      the conversion here.   Better:  standarize on either of the above for attributes.

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
pub trait IdentityAttributeStorageReader: Send + Sync + 'static {
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
pub trait IdentityAttributeStorageWriter: Send + Sync + 'static {
    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    async fn put_attributes(
        &self,
        identity: &IdentityIdentifier,
        entry: AttributesEntry,
    ) -> Result<()>;

    /// Remove all attributes for a given identity identifier
    async fn delete(&self, identity: &IdentityIdentifier) -> Result<()>;
}

/// Trait implementing read/write access to an AuthenticatedIdentities table
#[async_trait]
pub trait IdentityAttributeStorage:
    IdentityAttributeStorageReader + IdentityAttributeStorageWriter
{
    /// Return this storage as a read storage
    fn as_identity_attribute_storage_reader(&self) -> Arc<dyn IdentityAttributeStorageReader>;

    /// Return this storage as a write storage
    fn as_identity_attribute_storage_writer(&self) -> Arc<dyn IdentityAttributeStorageWriter>;
}

/// Implementation of `IdentityAttributeStorage` trait based on an underling
/// `AuthenticatedStorage` store.
#[derive(Clone)]
pub struct AuthenticatedAttributeStorage {
    storage: Arc<dyn AuthenticatedStorage>,
}

impl AuthenticatedAttributeStorage {
    /// Constructor. `AttributesEntry` entries are serialized and stored on the underling
    /// storage given.
    pub fn new(storage: Arc<dyn AuthenticatedStorage>) -> Self {
        Self { storage }
    }
}

impl IdentityAttributeStorage for AuthenticatedAttributeStorage {
    fn as_identity_attribute_storage_reader(&self) -> Arc<dyn IdentityAttributeStorageReader> {
        Arc::new(self.clone())
    }

    fn as_identity_attribute_storage_writer(&self) -> Arc<dyn IdentityAttributeStorageWriter> {
        Arc::new(self.clone())
    }
}

#[async_trait]
impl IdentityAttributeStorageReader for AuthenticatedAttributeStorage {
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
impl IdentityAttributeStorageWriter for AuthenticatedAttributeStorage {
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

    async fn delete(&self, identity: &IdentityIdentifier) -> Result<()> {
        self.storage
            .del(
                identity.to_string().as_str(),
                IdentityStateConst::ATTRIBUTES_KEY,
            )
            .await
    }
}

/// In-memory impl
pub mod mem;
