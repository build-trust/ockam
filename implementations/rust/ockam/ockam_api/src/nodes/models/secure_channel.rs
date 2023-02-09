use std::time::Duration;

use minicbor::{Decode, Encode};

use crate::nodes::registry::SecureChannelInfo;
use ockam_core::compat::borrow::Cow;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{route, Address, CowStr, Result};
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;
use serde::Serialize;

use crate::error::ApiError;
use crate::route_to_multiaddr;

#[derive(Debug, Clone, Copy, Decode, Encode)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum CredentialExchangeMode {
    #[n(0)] None,
    #[n(1)] Oneway,
    #[n(2)] Mutual,
}

/// Request body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6300395>,
    #[b(1)] pub addr: CowStr<'a>,
    #[b(2)] pub authorized_identifiers: Option<Vec<CowStr<'a>>>,
    #[n(3)] pub credential_exchange_mode: CredentialExchangeMode,
    #[n(4)] pub timeout: Option<Duration>,
    #[b(5)] pub identity: Option<CowStr<'a>>,
    #[n(6)] pub check_credentials: Option<bool>,
}

impl<'a> CreateSecureChannelRequest<'a> {
    pub fn new(
        addr: &MultiAddr,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        credential_exchange_mode: CredentialExchangeMode,
        check_credentials: Option<bool>,
        identity: Option<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string().into()).collect()),
            credential_exchange_mode,
            check_credentials,
            timeout: None,
            identity: identity.map(|x| x.into()),
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
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> CreateSecureChannelResponse<'a> {
    pub fn new(addr: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
        }
    }

    pub fn to_owned<'r>(&self) -> CreateSecureChannelResponse<'r> {
        CreateSecureChannelResponse {
            #[cfg(feature = "tag")]
            tag: self.tag.to_owned(),
            addr: self.addr.to_owned(),
        }
    }

    pub fn addr(&self) -> Result<MultiAddr> {
        route_to_multiaddr(&route![self.addr.to_string()])
            .ok_or_else(|| ApiError::generic(&format!("Invalid route: {}", self.addr)))
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
    #[b(3)] pub identity: Option<CowStr<'a>>,
}

impl<'a> CreateSecureChannelListenerRequest<'a> {
    pub fn new(
        addr: &Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        identity: Option<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string().into()).collect()),
            identity: identity.map(|x| x.into()),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8472592>,
    #[b(1)] pub channel: Cow<'a, str>,
}

impl<'a> DeleteSecureChannelRequest<'a> {
    pub fn new(channel: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: channel.to_string().into(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6953395>,
    #[b(1)] pub channel: Option<Cow<'a, str>>,
}

impl<'a> DeleteSecureChannelResponse<'a> {
    pub fn new(channel: Option<Address>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: channel.map(|ch| ch.to_string().into()),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3277982>,
    #[b(1)] pub channel: Cow<'a, str>,
}

impl<'a> ShowSecureChannelRequest<'a> {
    pub fn new(channel: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: channel.to_string().into(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode, Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelResponse<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<4566220>,
    #[b(1)] pub channel: Option<Cow<'a, str>>,
    #[b(2)] pub route: Option<Cow<'a, str>>,
    #[b(4)] pub authorized_identifiers: Option<Vec<CowStr<'a>>>,
}

impl<'a> ShowSecureChannelResponse<'a> {
    pub fn new(info: Option<&SecureChannelInfo>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: info.map(|info| info.addr().to_string().into()),
            route: info.map(|info| info.route().to_string().into()),
            authorized_identifiers: info
                .map(|info| {
                    info.authorized_identifiers()
                        .map(|ids| ids.iter().map(|iid| iid.to_string().into()).collect())
                })
                .unwrap_or(None),
        }
    }
}
