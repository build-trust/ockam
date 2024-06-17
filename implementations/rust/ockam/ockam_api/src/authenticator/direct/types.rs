use minicbor::{CborLen, Decode, Encode};
use ockam::identity::Identifier;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddMember {
    #[n(1)] member: Identifier,
    #[b(2)] attributes: BTreeMap<String, String>,
}

impl AddMember {
    pub fn new(member: Identifier) -> Self {
        AddMember {
            member,
            attributes: BTreeMap::new(),
        }
    }

    pub fn with_attributes(mut self, attributes: BTreeMap<String, String>) -> Self {
        self.attributes = attributes;
        self
    }

    pub fn member(&self) -> &Identifier {
        &self.member
    }

    pub fn attributes(&self) -> &BTreeMap<String, String> {
        &self.attributes
    }
}

#[derive(Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateToken {
    #[b(1)] attributes: BTreeMap<String, String>,
    #[n(2)] ttl_secs: Option<u64>,
    #[n(3)] ttl_count: Option<u64>,
}

impl CreateToken {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        CreateToken {
            attributes: Default::default(),
            ttl_count: None,
            ttl_secs: None,
        }
    }

    pub fn with_attributes(mut self, attributes: BTreeMap<String, String>) -> Self {
        self.attributes = attributes;
        self
    }

    pub fn with_ttl(mut self, duration: Option<Duration>) -> Self {
        self.ttl_secs = duration.map(|d| d.as_secs());
        self
    }

    pub fn with_ttl_count(mut self, ttl_count: Option<u64>) -> Self {
        self.ttl_count = ttl_count;
        self
    }

    pub fn into_owned_attributes(self) -> BTreeMap<String, String> {
        self.attributes.clone()
    }

    pub fn ttl_count(&self) -> Option<u64> {
        self.ttl_count
    }

    pub fn ttl_secs(&self) -> Option<u64> {
        self.ttl_secs
    }
}
