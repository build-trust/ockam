use minicbor::{Decode, Encode};
use ockam_core::CowStr;
use ockam_identity::IdentityIdentifier;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Enroller {}
