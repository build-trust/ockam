#![allow(missing_docs)]

use minicbor::{Decode, Encode};
use ockam_api::{CowBytes, CowStr};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3500430>,
    #[b(1)] identity_id: CowStr<'a>,
}

impl<'a> CreateResponse<'a> {
    pub fn new(identity_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id: identity_id.into(),
        }
    }
    pub fn identity_id(&self) -> &str {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ImportRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2308772>,
    #[b(1)] identity: CowBytes<'a>,
}

impl<'a> ImportRequest<'a> {
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
pub struct ImportResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3219065>,
    #[b(1)] identity_id: CowStr<'a>,
}

impl<'a> ImportResponse<'a> {
    pub fn new(identity_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id: identity_id.into(),
        }
    }
    pub fn identity_id(&self) -> &str {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ExportResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2421306>,
    #[b(1)] identity: CowBytes<'a>,
}

impl<'a> ExportResponse<'a> {
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
    #[b(1)] contact: CowBytes<'a>,
}

impl<'a> VerifyAndAddContactRequest<'a> {
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
pub struct CreateAuthProofRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1019956>,
    #[b(1)] state: CowBytes<'a>,
}

impl<'a> CreateAuthProofRequest<'a> {
    pub fn new(state: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            state: state.into(),
        }
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
    #[b(1)] identity_id: CowStr<'a>,
    #[b(2)] state: CowBytes<'a>,
    #[b(3)] proof: CowBytes<'a>,
}

impl<'a> VerifyAuthProofRequest<'a> {
    pub fn new(
        identity_id: impl Into<CowStr<'a>>,
        state: impl Into<CowBytes<'a>>,
        proof: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id: identity_id.into(),
            state: state.into(),
            proof: proof.into(),
        }
    }
    pub fn identity_id(&self) -> &str {
        &self.identity_id
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
