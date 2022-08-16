use minicbor::{Decode, Encode};
use ockam_identity::IdentityIdentifier;
use serde::{Deserialize, Serialize};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddMember {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2820828>,
    #[n(1)] member: IdentityIdentifier
}

impl AddMember {
    pub fn new(member: IdentityIdentifier) -> Self {
        AddMember {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            member,
        }
    }

    pub fn member(&self) -> &IdentityIdentifier {
        &self.member
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Enroller {}
