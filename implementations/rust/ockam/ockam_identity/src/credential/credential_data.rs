use crate::alloc::string::ToString;
use crate::identity::identity_change::IdentityChangeConstants;
use crate::identity::IdentityIdentifier;
use crate::CredentialBuilder;
use core::marker::PhantomData;
use core::time::Duration;
use minicbor::bytes::ByteVec;
use minicbor::{Decode, Encode};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::{collections::BTreeMap, fmt, string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use time::format_description::well_known::iso8601::{Iso8601, TimePrecision};
#[cfg(feature = "std")]
use time::{Error::Format, OffsetDateTime};

#[cfg(feature = "tag")]
use crate::TypeTag;

/// Identifier for the schema of a project credential
pub const PROJECT_MEMBER_SCHEMA: SchemaId = SchemaId(1);

/// Set of attributes associated to a given identity issued by another identity
#[derive(Debug, Decode, Encode, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData<T> {
    /// A schema identifier to allow distinguishing sets of attributes.
    #[n(1)] pub(crate) schema: Option<SchemaId>,
    /// User-defined key-value pairs.
    #[b(2)] pub(crate) attributes: Attributes,
    /// The subject this credential is issued for.
    #[n(3)] pub(crate) subject: IdentityIdentifier,
    /// The entity that signed this credential.
    #[n(4)] pub(crate) issuer: IdentityIdentifier,
    /// The label of the issuer's public key.
    #[b(5)] pub(crate) issuer_key_label: String,
    /// The time when this credential was created.
    #[n(6)] pub(crate) created: Timestamp,
    /// The time this credential expires.
    #[n(7)] pub(crate) expires: Timestamp,
    /// Term to represent the verification status type.
    #[n(8)] pub(crate) status: Option<PhantomData<T>>,
}

impl CredentialData<Verified> {
    /// Create a builder for a subject and an issuer, all other fields are optional and
    /// can be set with the builder methods
    pub fn builder(subject: IdentityIdentifier, issuer: IdentityIdentifier) -> CredentialBuilder {
        CredentialBuilder::new(subject, issuer)
    }

    /// Return a credential data struct with a fixed set of attributes
    pub fn from_attributes(
        subject: IdentityIdentifier,
        issuer: IdentityIdentifier,
        attrs: HashMap<String, String>,
    ) -> Result<CredentialData<Verified>> {
        CredentialBuilder::from_attributes(subject, issuer, attrs).build()
    }
}

impl CredentialData<Unverified> {
    pub(crate) fn verify(
        &self,
        subject: &IdentityIdentifier,
        issuer: &IdentityIdentifier,
        now: Timestamp,
    ) -> Result<()> {
        if self.issuer_key_label != IdentityChangeConstants::ROOT_LABEL {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "invalid signing key",
            ));
        }

        if &self.issuer != issuer {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "unknown authority",
            ));
        }

        if &self.subject != subject {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "unknown subject",
            ));
        }

        if self.expires <= now {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "expired credential",
            ));
        }

        Ok(())
    }

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

impl fmt::Display for SchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Maximum duration for a valid credential in seconds (30 days)
pub const MAX_CREDENTIAL_VALIDITY: Duration = Duration::from_secs(30 * 24 * 3600);

/// Type to represent data of verified credential.
#[derive(Debug, Encode, Clone)]
pub enum Verified {}

/// Type to represent data of unverified credential.
#[derive(Debug, Decode, Clone)]
pub enum Unverified {}

impl CredentialData<Verified> {
    /// Return the credential schema
    pub fn schema(&self) -> Option<SchemaId> {
        self.schema
    }

    /// Return the credential subject
    pub fn subject(&self) -> &IdentityIdentifier {
        &self.subject
    }

    /// Return the credential issuer
    pub fn issuer(&self) -> &IdentityIdentifier {
        &self.issuer
    }

    /// Return the credential issuer
    pub fn issuer_key_label(&self) -> &str {
        &self.issuer_key_label
    }

    /// Return the credential creation date
    pub fn created_at(&self) -> Timestamp {
        self.created
    }

    /// Return the credential expiration date
    pub fn expires_at(&self) -> Timestamp {
        self.expires
    }

    /// Return the identity attributes as a reference
    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// Return the identity attributes
    pub fn into_attributes(self) -> Attributes {
        self.attributes
    }
}

impl CredentialData<Unverified> {
    /// Return the issuer of a credential data when unverified
    pub fn unverified_issuer(&self) -> &IdentityIdentifier {
        &self.issuer
    }

    /// Return the issuer key label of a credential data when unverified
    pub fn unverified_key_label(&self) -> &str {
        &self.issuer_key_label
    }

    /// Return the subject of a credential data when unverified
    pub fn unverified_subject(&self) -> &IdentityIdentifier {
        &self.subject
    }
}

impl TryFrom<&[u8]> for CredentialData<Unverified> {
    type Error = minicbor::decode::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        minicbor::decode(value)
    }
}

/// User-defined key-value pairs.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4724285>,
    #[b(1)] attrs: BTreeMap<String, ByteVec>,
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

    /// Return true if this set of key / value is empty
    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    /// Return the number of key / values
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

    /// Return the value associated to a given key
    pub fn get(&self, k: &str) -> Option<&[u8]> {
        self.attrs.get(k).map(|s| &***s)
    }

    /// Return an iterator on the list of key / values
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ByteVec)> {
        self.attrs.iter()
    }

    //TODO: review the credential' attributes types.   They are references and has lifetimes,
    //etc,  but in reality this is always just deserizalided (either from wire or from
    //storage), so imho all that just add to the complexity without gaining much
    pub(crate) fn as_map_vec_u8(&self) -> BTreeMap<String, Vec<u8>> {
        self.attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_vec()))
            .collect()
    }
}

/// A Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
#[derive(
    Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cbor(transparent)]
pub struct Timestamp(#[n(0)] u64);

impl Timestamp {
    /// Create a new timestamp using the system time
    #[cfg(feature = "std")]
    pub fn now() -> Option<Self> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| Timestamp(d.as_secs()))
    }

    pub(crate) fn add_seconds(&self, seconds: u64) -> Self {
        Timestamp(self.0.saturating_add(seconds))
    }

    /// Create a new timestamp using the system time
    #[cfg(not(feature = "std"))]
    pub fn now() -> Option<Self> {
        None
    }

    /// Return the time elapsed between this timestamp and a previous one
    pub fn elapsed(&self, since: Timestamp) -> Option<Duration> {
        (self.0 >= since.0).then(|| Duration::from_secs(self.0 - since.0))
    }

    /// Return the timestamp value as a number of seconds since the UNIX epoch time
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

#[cfg(test)]
pub(crate) mod test {
    use super::*;

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

    pub(crate) fn make_credential_data() -> CredentialData<Verified> {
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
