use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

use ockam_core::CowBytes;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Response body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateIdentityResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2187575>,
    #[b(1)] pub identity_id: Cow<'a, str>,
}

impl<'a> CreateIdentityResponse<'a> {
    pub fn new(identity_id: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id: identity_id.into(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct LongIdentityResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7961643>,
    #[b(1)] pub identity: CowBytes<'a>,
}

impl<'a> LongIdentityResponse<'a> {
    pub fn new(identity: impl Into<Cow<'a, [u8]>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: CowBytes(identity.into()),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShortIdentityResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5773131>,
    #[b(1)] pub identity_id: Cow<'a, str>,
}

impl<'a> ShortIdentityResponse<'a> {
    pub fn new(identity_id: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id: identity_id.into(),
        }
    }
}
