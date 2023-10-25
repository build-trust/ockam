use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use ockam_core::CowStr;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddMember<'a> {
    #[n(1)] member: Identifier,
    #[b(2)] attributes: HashMap<CowStr<'a>, CowStr<'a>>,
}

impl<'a> AddMember<'a> {
    pub fn new(member: Identifier) -> Self {
        AddMember {
            member,
            attributes: HashMap::new(),
        }
    }

    pub fn with_attributes<S: Into<CowStr<'a>>>(mut self, attributes: HashMap<S, S>) -> Self {
        self.attributes = attributes
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    pub fn member(&self) -> &Identifier {
        &self.member
    }

    pub fn attributes(&self) -> &HashMap<CowStr, CowStr> {
        &self.attributes
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateToken<'a> {
    #[b(1)] attributes: HashMap<CowStr<'a>, CowStr<'a>>,
    #[n(2)] ttl_secs: Option<u64>,
    #[n(3)] ttl_count: Option<u64>,
}

impl<'a> CreateToken<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        CreateToken {
            attributes: HashMap::new(),
            ttl_count: None,
            ttl_secs: None,
        }
    }

    pub fn with_attributes<S: Into<CowStr<'a>>>(mut self, attributes: HashMap<S, S>) -> Self {
        self.attributes = attributes
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
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

    pub fn into_owned_attributes(self) -> HashMap<String, String> {
        self.attributes
            .into_iter()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    pub fn ttl_count(&self) -> Option<u64> {
        self.ttl_count
    }

    pub fn ttl_secs(&self) -> Option<u64> {
        self.ttl_secs
    }
}
