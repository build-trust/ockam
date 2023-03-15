#![allow(missing_docs)]

mod identity;
mod public_identity;
mod worker;

pub mod access_control;
pub mod one_time_code;

use ockam_core::compat::collections::HashMap;
pub use one_time_code::*;

use crate::IdentityIdentifier;
use core::fmt;
use core::marker::PhantomData;
use core::time::Duration;
use minicbor::bytes::ByteVec;
use minicbor::{Decode, Encode};
use ockam_core::compat::{collections::BTreeMap, string::String, vec::Vec};
use ockam_core::Result;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde::{Serialize, Serializer};
#[cfg(feature = "std")]
use time::format_description::well_known::iso8601::{Iso8601, TimePrecision};
#[cfg(feature = "std")]
use time::{Error::Format, OffsetDateTime};

#[cfg(feature = "std")]
use std::ops::Deref;

#[cfg(feature = "tag")]
use crate::TypeTag;

pub const MAX_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(30 * 24 * 3600);

/// Type to represent data of verified credential.
#[derive(Debug, Encode)]
pub enum Verified {}

/// Type to represent data of unverified credential.
#[derive(Debug, Decode)]
pub enum Unverified {}

#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3796735>,
    /// CBOR-encoded [`CredentialData`].
    #[cbor(with = "minicbor::bytes")]
    #[b(1)] data: Vec<u8>,
    /// Cryptographic signature of attributes data.
    #[cbor(with = "minicbor::bytes")]
    #[b(2)] signature: Vec<u8>,
}

impl fmt::Display for Credential {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = CredentialData::<Unverified>::try_from(self)
            .map_err(|_| fmt::Error)?
            .into_verified();
        write!(f, "{}", data)?;
        writeln!(f, "Signature:  {}", hex::encode(self.signature.deref()))
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Credential {{ ... }}")
    }
}

impl fmt::Display for CredentialData<Verified> {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use time::format_description::well_known::iso8601;

        if let Some(schema_id) = self.schema {
            writeln!(f, "Schema:     {schema_id}")?;
        }
        writeln!(f, "Subject:    {}", self.subject)?;
        writeln!(f, "Issuer:     {} ({})", self.issuer, self.issuer_key_label)?;

        let human_readable_time =
            |time: Timestamp| match OffsetDateTime::from_unix_timestamp(u64::from(time) as i64) {
                Ok(time) => {
                    match time.format(
                        &Iso8601::<
                            {
                                iso8601::Config::DEFAULT
                                    .set_time_precision(TimePrecision::Second {
                                        decimal_digits: None,
                                    })
                                    .encode()
                            },
                        >,
                    ) {
                        Ok(now_iso) => now_iso,
                        Err(_) => Format(time::error::Format::InvalidComponent("timestamp error"))
                            .to_string(),
                    }
                }
                Err(_) => Format(time::error::Format::InvalidComponent(
                    "unix time is invalid",
                ))
                .to_string(),
            };
        writeln!(f, "Created:    {}", human_readable_time(self.created))?;
        writeln!(f, "Expires:    {}", human_readable_time(self.expires))?;
        write!(f, "Attributes: ")?;
        f.debug_map()
            .entries(
                self.attributes
                    .iter()
                    .map(|(k, v)| (k, std::str::from_utf8(v).unwrap_or("**binary**"))),
            )
            .finish()?;
        writeln!(f)
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", self)
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData<T> {
    /// A schema identifier to allow distinguishing sets of attributes.
    #[n(1)] schema: Option<SchemaId>,
    /// User-defined key-value pairs.
    #[b(2)] attributes: Attributes,
    /// The subject this credential is issued for.
    #[n(3)] subject: IdentityIdentifier,
    /// The entity that signed this credential.
    #[n(4)] issuer: IdentityIdentifier,
    /// The label of the issuer's public key.
    #[b(5)] issuer_key_label: String,
    /// The time when this credential was created.
    #[n(6)] created: Timestamp,
    /// The time this credential expires.
    #[n(7)] expires: Timestamp,
    /// Term to represent the verification status type.
    #[n(8)] status: Option<PhantomData<T>>
}

impl CredentialData<Unverified> {
    pub(crate) fn into_verified(self) -> CredentialData<Verified> {
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

impl Credential {
    pub fn builder(subject: IdentityIdentifier) -> CredentialBuilder {
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

    fn new(data: Vec<u8>, signature: Vec<u8>) -> Self {
        Credential {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            data,
            signature,
        }
    }
}

impl CredentialData<Verified> {
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

    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    pub fn into_attributes(self) -> Attributes {
        self.attributes
    }
}

impl CredentialData<Unverified> {
    pub fn unverified_issuer(&self) -> &IdentityIdentifier {
        &self.issuer
    }
    pub fn unverified_key_label(&self) -> &str {
        &self.issuer_key_label
    }
    pub fn unverified_subject(&self) -> &IdentityIdentifier {
        &self.subject
    }
}

impl TryFrom<&Credential> for CredentialData<Unverified> {
    type Error = minicbor::decode::Error;

    fn try_from(value: &Credential) -> Result<Self, Self::Error> {
        minicbor::decode(value.clone().data.as_slice())
    }
}

/// User-defined key-value pairs.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4724285>,

    #[b(1)] attrs: BTreeMap<String, ByteVec>
}

impl Attributes {
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
    pub fn put(&mut self, k: &str, v: &[u8]) -> &mut Self {
        self.attrs.insert(k.into(), v.to_vec().into());
        self
    }

    pub fn get(&self, k: &str) -> Option<&[u8]> {
        self.attrs.get(k).map(|s| &***s)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ByteVec)> {
        self.attrs.iter()
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

    pub fn unix_time(&self) -> u64 {
        self.0
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

impl fmt::Display for SchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Convenience structure to create [`Credential`]s.
pub struct CredentialBuilder {
    schema: Option<SchemaId>,
    attrs: Attributes,
    subject: IdentityIdentifier,
    validity: Duration,
}

impl CredentialBuilder {
    pub fn from_attributes(identity: IdentityIdentifier, attrs: HashMap<String, String>) -> Self {
        attrs
            .iter()
            .fold(Credential::builder(identity), |crd, (k, v)| {
                crd.with_attribute(k, v.as_bytes())
            })
    }
    /// Add some key-value pair as credential attribute.
    pub fn with_attribute(mut self, k: &str, v: &[u8]) -> Self {
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

impl Serialize for Credential {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let bytes = minicbor::to_vec(self).expect("encoding credential to vec never errors");
        if ser.is_human_readable() {
            ser.serialize_str(&hex::encode(&bytes))
        } else {
            ser.serialize_bytes(&bytes)
        }
    }
}

impl<'a> Deserialize<'a> for Credential {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        let bytes: Vec<u8> = if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            hex::decode(s).map_err(D::Error::custom)?
        } else {
            <Vec<u8>>::deserialize(deserializer)?
        };
        minicbor::decode(&bytes).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use serde_json;

    #[quickcheck]
    fn test_serialization_roundtrip(credential: Credential) -> bool {
        let serialized = serde_bare::to_vec(&credential).unwrap();
        let actual: Credential = serde_bare::from_slice(serialized.as_slice()).unwrap();
        actual == credential
    }

    #[test]
    fn test_serialization() {
        // this test makes sure that we are using the minicbor Bytes encoder
        // for the Credential fields
        let credential = Credential::new(vec![1, 2, 3], vec![5, 6, 7]);
        let serialized = serde_bare::to_vec(&credential).unwrap();
        let expected: Vec<u8> = vec![11, 162, 1, 67, 1, 2, 3, 2, 67, 5, 6, 7];
        assert_eq!(serialized, expected)
    }

    #[quickcheck]
    fn test_serialization_roundtrip_human_readable(credential: Credential) -> bool {
        let serialized = serde_json::to_string(&credential).unwrap();
        let actual: Credential = serde_json::from_str(serialized.as_str()).unwrap();
        actual == credential
    }

    impl Arbitrary for Credential {
        fn arbitrary(g: &mut Gen) -> Self {
            Credential::new(<Vec<u8>>::arbitrary(g), <Vec<u8>>::arbitrary(g))
        }

        /// there is no meaningful shrinking in general for a credential
        fn shrink(&self) -> Box<dyn Iterator<Item = Credential>> {
            Box::new(std::iter::empty())
        }
    }

    #[test]
    fn test_display_credential_data() {
        let credential_data = make_credential_data();
        let actual = format!("{credential_data}");
        let expected = r#"Schema:     1
Subject:    P6474cfdbf547240b6d716bff89c976810859bc3f47be8ea620df12a392ea6cb7
Issuer:     P0db4fec87ff764485f1311e68d6f474e786f1fdbafcd249a5eb73dd681fd1d5d (OCKAM_RK)
Created:    1970-01-01T00:02:00Z
Expires:    1970-01-01T00:03:20Z
Attributes: {"name": "value"}
"#;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_display_credential() {
        let credential_data = make_credential_data();
        let data = minicbor::to_vec(credential_data).unwrap();
        let credential = Credential::new(data, vec![1, 2, 3]);

        let actual = format!("{credential}");
        let expected = r#"Schema:     1
Subject:    P6474cfdbf547240b6d716bff89c976810859bc3f47be8ea620df12a392ea6cb7
Issuer:     P0db4fec87ff764485f1311e68d6f474e786f1fdbafcd249a5eb73dd681fd1d5d (OCKAM_RK)
Created:    1970-01-01T00:02:00Z
Expires:    1970-01-01T00:03:20Z
Attributes: {"name": "value"}
Signature:  010203
"#;
        assert_eq!(actual, expected);
    }

    fn make_credential_data() -> CredentialData<Verified> {
        let mut attributes = Attributes::new();
        attributes.put("name", "value".as_bytes());

        CredentialData {
            schema: Some(SchemaId(1)),
            subject: IdentityIdentifier::from_key_id(
                "6474cfdbf547240b6d716bff89c976810859bc3f47be8ea620df12a392ea6cb7",
            ),
            issuer: IdentityIdentifier::from_key_id(
                "0db4fec87ff764485f1311e68d6f474e786f1fdbafcd249a5eb73dd681fd1d5d",
            ),
            attributes,
            issuer_key_label: "OCKAM_RK".into(),
            created: Timestamp(120),
            expires: Timestamp(200),
            status: None::<PhantomData<Verified>>,
        }
    }
}
