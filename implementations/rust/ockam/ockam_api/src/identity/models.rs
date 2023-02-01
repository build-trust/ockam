#![allow(missing_docs)]

use ockam_core::{CowBytes, CowStr};

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
pub struct ValidateIdentityChangeHistoryResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4245404>,
    #[b(1)] identity_id: CowStr<'a>,
}

impl<'a> ValidateIdentityChangeHistoryResponse<'a> {
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

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSignatureRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1019956>,
    #[b(1)] identity: CowBytes<'a>,
    #[b(2)] data: CowBytes<'a>,
    #[b(3)] vault_name: Option<CowStr<'a>>,
}

impl<'a> CreateSignatureRequest<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>, data: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            data: data.into(),
            vault_name: None,
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn vault_name(&self) -> Option<String> {
        self.vault_name.as_ref().map(|x| x.to_string())
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSignatureResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2592832>,
    #[b(1)] signature: CowBytes<'a>,
}

impl<'a> CreateSignatureResponse<'a> {
    pub fn new(signature: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            signature: signature.into(),
        }
    }
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifySignatureRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7550780>,
    #[b(1)] signer_identity: CowBytes<'a>,
    #[b(2)] data: CowBytes<'a>,
    #[b(3)] signature: CowBytes<'a>,
}

impl<'a> VerifySignatureRequest<'a> {
    pub fn new(
        signer_identity: impl Into<CowBytes<'a>>,
        data: impl Into<CowBytes<'a>>,
        signature: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            signer_identity: signer_identity.into(),
            data: data.into(),
            signature: signature.into(),
        }
    }
    pub fn signer_identity(&self) -> &[u8] {
        &self.signer_identity
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifySignatureResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1236745>,
    #[n(1)] verified: bool,
}

impl VerifySignatureResponse {
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
