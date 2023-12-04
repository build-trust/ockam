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
    // TODO: Check how it looks serialized with both serde and minicbor
    #[b(1)] attrs: BTreeMap<Vec<u8>, Vec<u8>>,
    #[n(2)] added: TimestampInSeconds,
    #[n(3)] expires: Option<TimestampInSeconds>,
    #[n(4)] attested_by: Option<Identifier>,
}

impl AttributesEntry {
    //TODO: since we are converting from HashMap to BTreeMap in different parts,
    //      it will make sense to have a constructor here taking a HashMap and doing
    //      the conversion here.   Better:  standardize on either of the above for attributes.

    /// Constructor
    pub fn new(
        attrs: BTreeMap<Vec<u8>, Vec<u8>>,
        added: TimestampInSeconds,
        expires: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
    ) -> Self {
        Self {
            attrs,
            added,
            expires,
            attested_by,
        }
    }

    /// The entry attributes
    pub fn attrs(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.attrs
    }

    /// Expiration time for this entry
    pub fn expires(&self) -> Option<TimestampInSeconds> {
        self.expires
    }

    /// Date that the entry was added
    pub fn added(&self) -> TimestampInSeconds {
        self.added
    }

    /// Who attested this attributes for this identity identifier
    pub fn attested_by(&self) -> Option<Identifier> {
        self.attested_by.to_owned()
    }
}

impl AttributesEntry {
    /// Create an AttributesEntry with just one name/value pair
    pub fn singleton(
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
        expires: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
    ) -> Result<Self> {
        let attrs = BTreeMap::from([(attribute_name, attribute_value)]);
        Ok(Self {
            attrs,
            added: now()?,
            expires,
            attested_by,
        })
    }
}
