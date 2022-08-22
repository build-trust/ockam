#![allow(missing_docs)]

mod identity;
mod public_identity;
mod storage_utils;
mod worker;

pub use storage_utils::*;

use crate::IdentityIdentifier;
use core::fmt;
use core::marker::PhantomData;
use core::time::Duration;
use minicbor::bytes::ByteSlice;
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::compat::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{CowBytes, CowStr, Result};
use serde::{Serialize, Serializer};

#[cfg(feature = "tag")]
use crate::TypeTag;

pub const MAX_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(30 * 24 * 3600);

/// Type to represent data of verified credentials.
#[derive(Debug, Encode)]
pub enum Verified {}

/// Type to represent data of unverified credentials.
#[derive(Debug, Decode)]
pub enum Unverified {}

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3796735>,
    /// CBOR-encoded [`CredentialData`].
    #[b(1)] data: CowBytes<'a>,
    /// Cryptographic signature of attributes data.
    #[b(2)] signature: CowBytes<'a>,
}

impl fmt::Display for Credential<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData<'a, T> {
    /// A schema identifier to allow distinguishing sets of attributes.
    #[n(1)] schema: Option<SchemaId>,
    /// User-defined key-value pairs.
    #[b(2)] attributes: Attributes<'a>,
    /// The subject this credential is issued for.
    #[n(3)] subject: IdentityIdentifier,
    /// The entity that signed this credential.
    #[n(4)] issuer: IdentityIdentifier,
    /// The label of the issuer's public key.
    #[b(5)] issuer_key_label: CowStr<'a>,
    /// The time when this credential was created.
    #[n(6)] created: Timestamp,
    /// The time this credential expires.
    #[n(7)] expires: Timestamp,
    /// Term to represent the verification status type.
    #[n(8)] status: Option<PhantomData<T>>
}

impl<'a> CredentialData<'a, Unverified> {
    pub(crate) fn make_verified(self) -> CredentialData<'a, Verified> {
        CredentialData {
            schema: self.schema,
            attributes: self.attributes,
            subject: self.subject,
            issuer: self.issuer,
            issuer_key_label: self.issuer_key_label,
            created: self.created,
            expires: self.expires,
            status: None::<PhantomData<Verified>>,
        }
    }
}

impl<'a> Credential<'a> {
    pub fn builder<'b>(subject: IdentityIdentifier) -> CredentialBuilder<'b> {
        CredentialBuilder {
            schema: None,
            subject,
            attrs: Attributes::new(),
            validity: MAX_CREDENTIAL_VALIDITY,
        }
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    pub fn unverified_data(&self) -> &[u8] {
        &self.data
    }

    fn new<A, S>(data: A, signature: S) -> Self
    where
        A: Into<Cow<'a, [u8]>>,
        S: Into<Cow<'a, [u8]>>,
    {
        Credential {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            data: CowBytes(data.into()),
            signature: CowBytes(signature.into()),
        }
    }
}

impl<'a> CredentialData<'a, Verified> {
    pub fn schema(&self) -> Option<SchemaId> {
        self.schema
    }

    pub fn subject(&self) -> &IdentityIdentifier {
        &self.subject
    }

    pub fn issuer(&self) -> &IdentityIdentifier {
        &self.issuer
    }

    pub fn issuer_key_label(&self) -> &str {
        &self.issuer_key_label
    }

    pub fn created_at(&self) -> Timestamp {
        self.created
    }

    pub fn expires_at(&self) -> Timestamp {
        self.expires
    }

    pub fn attributes(&self) -> &Attributes<'_> {
        &self.attributes
    }

    pub fn into_attributes(self) -> Attributes<'a> {
        self.attributes
    }
}

impl<'a> CredentialData<'a, Unverified> {
    pub fn unverfied_issuer(&self) -> &IdentityIdentifier {
        &self.issuer
    }
    pub fn unverfied_key_label(&self) -> &str {
        &self.issuer_key_label
    }
}

impl<'a, 'b: 'a> TryFrom<&'b Credential<'a>> for CredentialData<'a, Unverified> {
    type Error = minicbor::decode::Error;

    fn try_from(value: &'b Credential<'a>) -> Result<Self, Self::Error> {
        minicbor::decode(&value.data)
    }
}

/// User-defined key-value pairs.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4724285>,
    #[b(1)] attrs: BTreeMap<&'a str, &'a ByteSlice>
}

impl<'a> Attributes<'a> {
    /// Create a new empty attribute set.
    pub fn new() -> Self {
        Attributes {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attrs: BTreeMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.attrs.len()
    }

    /// Add a key-value pair to the attribute set.
    ///
    /// If an entry with the same key exists it is replaced with the new value.
    pub fn put(&mut self, k: &'a str, v: &'a [u8]) -> &mut Self {
        self.attrs.insert(k, v.into());
        self
    }

    pub fn get(&self, k: &str) -> Option<&[u8]> {
        self.attrs.get(k).map(|s| &***s)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.attrs.iter().map(|(k, v)| (*k, &***v))
    }

    pub fn to_owned(&self) -> BTreeMap<String, Vec<u8>> {
        let mut map = BTreeMap::default();
        for (k, v) in self.iter() {
            map.insert(k.to_string(), v.to_vec());
        }

        map
    }
}

/// A Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cbor(transparent)]
pub struct Timestamp(#[n(0)] u64);

impl Timestamp {
    #[cfg(feature = "std")]
    pub fn now() -> Option<Self> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| Timestamp(d.as_secs()))
    }

    #[cfg(not(feature = "std"))]
    pub fn now() -> Option<Self> {
        None
    }

    pub fn elapsed(&self, since: Timestamp) -> Option<Duration> {
        (self.0 >= since.0).then(|| Duration::from_secs(self.0 - since.0))
    }
}

impl From<Timestamp> for u64 {
    fn from(t: Timestamp) -> Self {
        t.0
    }
}

/// A schema identifier allows discriminate sets of credential attributes.
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cbor(transparent)]
pub struct SchemaId(#[n(0)] pub u64);

impl From<SchemaId> for u64 {
    fn from(s: SchemaId) -> Self {
        s.0
    }
}

/// Convenience structure to create [`Credential`]s.
pub struct CredentialBuilder<'a> {
    schema: Option<SchemaId>,
    attrs: Attributes<'a>,
    subject: IdentityIdentifier,
    validity: Duration,
}

impl<'a> CredentialBuilder<'a> {
    /// Add some key-value pair as credential attribute.
    pub fn with_attribute(mut self, k: &'a str, v: &'a [u8]) -> Self {
        self.attrs.put(k, v);
        self
    }

    /// Set the schema identifier of the credential.
    pub fn with_schema(mut self, s: SchemaId) -> Self {
        self.schema = Some(s);
        self
    }

    /// Set validity duration of the credential.
    ///
    /// # Panics
    ///
    /// If the given validity exceeds [`MAX_CREDENTIAL_VALIDITY`].
    pub fn valid_for(mut self, val: Duration) -> Self {
        assert! {
            val <= MAX_CREDENTIAL_VALIDITY,
            "validity exceeds allowed maximum"
        }
        self.validity = val;
        self
    }
}

impl Serialize for Credential<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let bytes = minicbor::to_vec(self).expect("encoding credential to vec never errors");
        if ser.is_human_readable() {
            ser.serialize_str(&hex::encode(&bytes))
        } else {
            ser.serialize_bytes(&bytes)
        }
    }
}
