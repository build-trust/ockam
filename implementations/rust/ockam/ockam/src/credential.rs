#![allow(dead_code, missing_docs, unused_imports)]

//! Building block for Attribute-based access control

// TODO: std is needed to check credential expiration. Figure out what can be done here
#[cfg(feature = "std")]
mod exchange;
#[cfg(feature = "std")]
pub use exchange::*;

use core::marker::PhantomData;
use core::time::Duration;
use minicbor::bytes::ByteSlice;
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{Signature, SignatureVec, Verifier};
use ockam_core::{CowBytes, CowStr, Error, Result};
use ockam_identity::change_history::IdentityChangeHistory;
use ockam_identity::{Identity, IdentityIdentifier, IdentityStateConst, IdentityVault};

#[cfg(feature = "tag")]
use crate::TypeTag;

pub const MAX_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(6 * 3600);

/// Type to represent data of verified credentials.
#[derive(Debug, Encode)]
pub enum Verified {}

/// Type to represent data of unverified credentials.
#[derive(Debug, Decode)]
pub enum Unverified {}

#[derive(Debug, Decode, Encode)]
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
    #[b(5)] issuer_key: CowStr<'a>,
    /// The time when this credential was created.
    #[n(6)] created: Timestamp,
    /// The time this credential expires.
    #[n(7)] expires: Timestamp,
    /// Term to represent the verification status type.
    #[n(8)] status: Option<PhantomData<T>>
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

    /// Perform a signature check with the given identity.
    ///
    /// If successful, the credential data are returned.
    pub async fn verify_signature<'b: 'a, V>(
        &'b self,
        issuer: &IdentityChangeHistory,
        verifier: V,
    ) -> Result<CredentialData<'a, Verified>>
    where
        V: Verifier,
    {
        let dat = CredentialData::try_from(self)?;
        if dat.issuer_key != IdentityStateConst::ROOT_LABEL {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "invalid signing key",
            ));
        }
        let sig = Signature::new(self.signature.clone().into_owned());
        let pky = issuer.get_public_key(&dat.issuer_key)?;
        if !verifier.verify(&sig, &pky, &self.data).await? {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "invalid signature",
            ));
        }
        Ok(CredentialData {
            schema: dat.schema,
            attributes: dat.attributes,
            subject: dat.subject,
            issuer: dat.issuer,
            issuer_key: dat.issuer_key,
            created: dat.created,
            expires: dat.expires,
            status: None::<PhantomData<Verified>>,
        })
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
        &self.issuer_key
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
            "validitiy exceeds allowed maximum"
        }
        self.validity = val;
        self
    }

    /// Create a signed credential based on the given values.
    #[cfg(feature = "std")]
    pub async fn issue<'b, V>(self, issuer: &Identity<V>) -> Result<Credential<'b>>
    where
        V: IdentityVault,
    {
        let key_label = IdentityStateConst::ROOT_LABEL;
        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Core, Kind::Internal, "invalid system time"))?;
        let exp = Timestamp(u64::from(now).saturating_add(self.validity.as_secs()));
        let dat = CredentialData {
            schema: self.schema,
            attributes: self.attrs,
            subject: self.subject,
            issuer: issuer.identifier().clone(),
            issuer_key: CowStr(key_label.into()),
            created: now,
            expires: exp,
            status: None::<PhantomData<Verified>>,
        };
        let bytes = minicbor::to_vec(&dat)?;
        let skey = issuer.get_secret_key(key_label).await?;
        let sig = issuer.vault().sign(&skey, &bytes).await?;
        Ok(Credential::new(bytes, SignatureVec::from(sig)))
    }
}
