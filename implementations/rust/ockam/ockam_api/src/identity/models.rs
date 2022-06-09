#![allow(missing_docs)]

use crate::{CowBytes, CowStr};
use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3500430>,
    #[b(1)] identity: CowBytes<'a>,
    #[b(2)] identity_id: CowStr<'a>,
}

impl<'a> CreateResponse<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>, identity_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            identity_id: identity_id.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn identity_id(&self) -> &str {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ContactRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9274303>,
    #[b(1)] identity: CowBytes<'a>,
}

impl<'a> ContactRequest<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ContactResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3682917>,
    #[b(1)] contact: CowBytes<'a>,
}

impl<'a> ContactResponse<'a> {
    pub fn new(contact: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            contact: contact.into(),
        }
    }
    pub fn contact(&self) -> &[u8] {
        &self.contact
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyAndAddContactRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3396927>,
    #[b(1)] identity: CowBytes<'a>,
    #[b(2)] contact: CowBytes<'a>,
}

impl<'a> VerifyAndAddContactRequest<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>, contact: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            contact: contact.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn contact(&self) -> &[u8] {
        &self.contact
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyAndAddContactResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7946005>,
    #[b(1)] identity: CowBytes<'a>,
    #[b(2)] contact_id: CowStr<'a>,
}

impl<'a> VerifyAndAddContactResponse<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>, contact_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            contact_id: contact_id.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn contact_id(&self) -> &str {
        &self.contact_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateAuthProofRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1019956>,
    #[b(1)] identity: CowBytes<'a>,
    #[b(2)] state: CowBytes<'a>,
}

impl<'a> CreateAuthProofRequest<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>, state: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            state: state.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn state(&self) -> &[u8] {
        &self.state
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateAuthProofResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2592832>,
    #[b(1)] proof: CowBytes<'a>,
}

impl<'a> CreateAuthProofResponse<'a> {
    pub fn new(proof: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            proof: proof.into(),
        }
    }
    pub fn proof(&self) -> &[u8] {
        &self.proof
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyAuthProofRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7550780>,
    #[b(1)] identity: CowBytes<'a>,
    #[b(2)] peer_identity_id: CowStr<'a>,
    #[b(3)] state: CowBytes<'a>,
    #[b(4)] proof: CowBytes<'a>,
}

impl<'a> VerifyAuthProofRequest<'a> {
    pub fn new(
        identity: impl Into<CowBytes<'a>>,
        peer_identity_id: impl Into<CowStr<'a>>,
        state: impl Into<CowBytes<'a>>,
        proof: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            peer_identity_id: peer_identity_id.into(),
            state: state.into(),
            proof: proof.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn peer_identity_id(&self) -> &str {
        &self.peer_identity_id
    }
    pub fn state(&self) -> &[u8] {
        &self.state
    }
    pub fn proof(&self) -> &[u8] {
        &self.proof
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyAuthProofResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1236745>,
    #[n(1)] verified: bool,
}

impl VerifyAuthProofResponse {
    pub fn new(verified: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            verified,
        }
    }
    pub fn verified(&self) -> bool {
        self.verified
    }
}
