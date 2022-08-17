use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::Address;

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;

/// Request body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6300395>,
    #[b(1)] pub addr: Cow<'a, str>,
    #[b(2)] pub authorized_identifiers: Option<Vec<CowStr<'a>>>,
}

impl<'a> CreateSecureChannelRequest<'a> {
    pub fn new(addr: &MultiAddr, authorized_identifiers: Option<Vec<IdentityIdentifier>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string().into()).collect()),
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
    pub fn new(addr: &Address) -> Self {
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
    #[b(2)] pub authorized_identifiers: Option<Vec<CowStr<'a>>>,
}

impl<'a> CreateSecureChannelListenerRequest<'a> {
    pub fn new(addr: &Address, authorized_identifiers: Option<Vec<IdentityIdentifier>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string().into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SecureChannelListenerListRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8008758>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl SecureChannelListenerListRequest<'_> {
    pub fn new(addr: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SecureChannelListenerList {
//    #[cfg(feature = "tag")]
//    #[n(0)] tag: TypeTag<5212817>,
//#[n(1)] pub list: Vec<String>
    #[n(0)] pub list: Vec<String>
}

impl SecureChannelListenerList {
    pub fn new(list: Vec<String>) -> Self {
        Self {
            //            #[cfg(feature = "tag")]
            //            tag: TypeTag,
            list,
        }
    }
}
