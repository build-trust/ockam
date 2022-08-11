use crate::signer::types::IdentityId;
use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddEnroller<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1010815>,
    #[b(1)] enroller: IdentityId<'a>
}

impl<'a> AddEnroller<'a> {
    pub fn new(enroller: IdentityId<'a>) -> Self {
        AddEnroller {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            enroller,
        }
    }

    pub fn enroller(&self) -> &IdentityId {
        &self.enroller
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddMember<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2820828>,
    #[b(1)] member: IdentityId<'a>
}

impl<'a> AddMember<'a> {
    pub fn new(member: IdentityId<'a>) -> Self {
        AddMember {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            member,
        }
    }

    pub fn member(&self) -> &IdentityId {
        &self.member
    }
}

/// Used until we know what enroller/member data we want to store.
#[derive(Debug, Decode, Encode)]
#[cbor(map)]
pub struct Placeholder;
