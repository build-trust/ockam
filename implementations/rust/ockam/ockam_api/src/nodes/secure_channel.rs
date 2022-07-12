use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::Address;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

/// Request body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6300395>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> CreateSecureChannelRequest<'a> {
    pub fn new(addr: MultiAddr) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
        }
    }
}

/// Response body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6056513>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> CreateSecureChannelResponse<'a> {
    pub fn new(addr: Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
        }
    }
}

/// Request body when instructing a node to create a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelListenerRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8112242>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> CreateSecureChannelListenerRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}
