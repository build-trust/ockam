use minicbor::{Decode, Encode};
use ockam_core::CowStr;
use ockam_identity::IdentityIdentifier;
use std::collections::HashMap;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddMember<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2820828>,
    #[n(1)] member: IdentityIdentifier,
    #[b(2)] attributes: HashMap<CowStr<'a>, CowStr<'a>>,
}

impl<'a> AddMember<'a> {
    pub fn new(member: IdentityIdentifier) -> Self {
        AddMember {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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

    pub fn member(&self) -> &IdentityIdentifier {
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
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2502742>,
    #[b(1)] attributes: HashMap<CowStr<'a>, CowStr<'a>>,
}

impl<'a> CreateToken<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        CreateToken {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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

    pub fn into_owned_attributes(self) -> HashMap<String, String> {
        self.attributes
            .into_iter()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }
}
