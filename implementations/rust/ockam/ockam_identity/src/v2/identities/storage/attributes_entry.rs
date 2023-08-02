use super::super::super::models::{Identifier, TimestampInSeconds};
use crate::alloc::borrow::ToOwned;
use minicbor::{Decode, Encode};
use ockam_core::compat::{collections::BTreeMap, string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// An entry on the AuthenticatedIdentities table.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry {
    #[b(1)] attrs: BTreeMap<String, Vec<u8>>,
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
        attrs: BTreeMap<String, Vec<u8>>,
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
    pub fn attrs(&self) -> &BTreeMap<String, Vec<u8>> {
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
