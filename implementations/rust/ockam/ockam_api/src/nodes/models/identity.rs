use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use serde::Serialize;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Decode, Encode, Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct LongIdentityResponse {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<7961643>,
    #[b(1)] pub identity_change_history: Vec<u8>,
}

impl LongIdentityResponse {
    pub fn new(identity_change_history: Vec<u8>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_change_history,
        }
    }
}

#[derive(Debug, Clone, Decode, Encode, Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShortIdentityResponse {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<5773131>,
    #[b(1)] pub identity_id: Identifier,
}

impl ShortIdentityResponse {
    pub fn new(identity_id: Identifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id,
        }
    }
}
