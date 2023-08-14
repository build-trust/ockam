#![allow(missing_docs)]

use ockam_core::CowBytes;

use minicbor::{Decode, Encode};
use ockam::identity::Identifier;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3500430>,
    #[n(1)] identity: Vec<u8>,
    #[n(2)] identity_id: Identifier,
}

impl CreateResponse {
    pub fn new(identity: Vec<u8>, identity_id: Identifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity,
            identity_id,
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ValidateIdentityChangeHistoryRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2556809>,
    #[b(1)] identity: CowBytes<'a>,
}

impl<'a> ValidateIdentityChangeHistoryRequest<'a> {
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
pub struct ValidateIdentityChangeHistoryResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4245404>,
    #[n(1)] identity_id: Identifier,
}

impl ValidateIdentityChangeHistoryResponse {
    pub fn new(identity_id: Identifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id,
        }
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CompareIdentityChangeHistoryRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7300740>,
    #[b(1)] current_identity: CowBytes<'a>,
    #[b(2)] known_identity: CowBytes<'a>,
}

impl<'a> CompareIdentityChangeHistoryRequest<'a> {
    pub fn new(
        current_identity: impl Into<CowBytes<'a>>,
        known_identity: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            current_identity: current_identity.into(),
            known_identity: known_identity.into(),
        }
    }
    pub fn current_identity(&self) -> &[u8] {
        &self.current_identity
    }
    pub fn known_identity(&self) -> &[u8] {
        &self.known_identity
    }
}
