use crate::models::{Identifier, TimestampInSeconds};
use crate::utils::now;
use crate::AttributeName;
use crate::AttributeValue;
use alloc::collections::btree_map::Iter;
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::ToOwned;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::Result;
use serde::{Deserialize, Serialize};

/// An entry on the AuthenticatedIdentities table.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry {
    // TODO: Check how it looks serialized with both serde and minicbor
    #[n(1)] attrs: BTreeMap<AttributeName, AttributeValue>,
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
        attrs: BTreeMap<AttributeName, AttributeValue>,
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

    /// Create an empty set of attributes with no dates or attestor
    pub fn empty() -> Result<Self> {
        Ok(Self::new(BTreeMap::default(), now()?, None, None))
    }

    /// Create an empty set of attributes with an identity which will attest additional attributes
    pub fn empty_attested_by(identifier: Identifier) -> Result<Self> {
        Ok(Self::new(
            BTreeMap::default(),
            now()?,
            None,
            Some(identifier),
        ))
    }

    /// Get the number of attributes
    pub fn len(&self) -> usize {
        self.attrs.len()
    }

    /// Return true if there are no attributes
    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    /// Get an attribute value by name
    pub fn get(&self, name: &AttributeName) -> Option<&AttributeValue> {
        self.attrs.get(name)
    }

    /// Get an attribute value by name
    pub fn insert(&mut self, name: AttributeName, value: AttributeValue) -> Option<AttributeValue> {
        self.attrs.insert(name, value)
    }

    /// The entry attributes
    pub fn iter(&self) -> Iter<AttributeName, AttributeValue> {
        self.attrs.iter()
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
