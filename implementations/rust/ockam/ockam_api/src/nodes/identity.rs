use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

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
