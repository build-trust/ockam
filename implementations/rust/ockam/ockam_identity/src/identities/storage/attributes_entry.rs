use crate::models::{Identifier, TimestampInSeconds};
use crate::utils::now;
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::ToOwned;
use ockam_core::compat::{collections::BTreeMap, vec::Vec};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

/// An entry on the AuthenticatedIdentities table.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry {
    #[n(1)] attrs: BTreeMap<Vec<u8>, Vec<u8>>,
    #[n(2)] added_at: TimestampInSeconds,
    #[n(3)] expires_at: Option<TimestampInSeconds>,
    #[n(4)] attested_by: Option<Identifier>,
}

impl AttributesEntry {
    /// Constructor
    pub fn new(
        attrs: BTreeMap<Vec<u8>, Vec<u8>>,
        added_at: TimestampInSeconds,
        expires_at: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
    ) -> Self {
        Self {
            attrs,
            added_at,
            expires_at,
            attested_by,
        }
    }

    /// The entry attributes
    pub fn attrs(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.attrs
    }

    /// Expiration time for this entry
    pub fn expires_at(&self) -> Option<TimestampInSeconds> {
        self.expires_at
    }

    /// Date that the entry was added
    pub fn added_at(&self) -> TimestampInSeconds {
        self.added_at
    }

    /// Who attested this attributes for this identity identifier
    pub fn attested_by(&self) -> Option<Identifier> {
        self.attested_by.to_owned()
    }
}

impl AttributesEntry {
    /// Create an AttributesEntry with just one name/value pair
    pub fn single(
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
        expires_at: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
    ) -> Result<Self> {
        let attrs = BTreeMap::from([(attribute_name, attribute_value)]);
        Ok(Self {
            attrs,
            added_at: now()?,
            expires_at,
            attested_by,
        })
    }
}
