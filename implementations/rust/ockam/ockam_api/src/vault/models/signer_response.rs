use crate::CowBytes;
use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SignResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2236127>,
    #[b(1)] signature: CowBytes<'a>,
}

impl<'a> SignResponse<'a> {
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
pub struct VerifyResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9171606>,
    #[n(1)] verified: bool,
}

impl VerifyResponse {
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
