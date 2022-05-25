use minicbor::{Decode, Encode};
use ockam_api::CowStr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct EcdhResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1455286>,
    #[b(1)] key_id: CowStr<'a>,
}

impl<'a> EcdhResponse<'a> {
    pub fn new(key_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            key_id: key_id.into(),
        }
    }
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ComputeKeyIdResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6264098>,
    #[b(1)] key_id: CowStr<'a>,
}

impl<'a> ComputeKeyIdResponse<'a> {
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn new(key_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            key_id: key_id.into(),
        }
    }
}
