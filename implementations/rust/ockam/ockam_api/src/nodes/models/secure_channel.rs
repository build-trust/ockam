use std::time::Duration;

use minicbor::{Decode, Encode};

use crate::nodes::registry::{SecureChannelInfo, SecureChannelListenerInfo};
use ockam::identity::IdentityIdentifier;
use ockam_core::compat::borrow::Cow;
use ockam_core::flow_control::FlowControlId;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{route, Address, CowStr, Result};
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
    #[b(5)] pub identity_name: Option<CowStr<'a>>,
    #[b(6)] pub credential_name: Option<CowStr<'a>>,
}

impl<'a> CreateSecureChannelRequest<'a> {
    pub fn new(
        addr: &MultiAddr,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        credential_exchange_mode: CredentialExchangeMode,
        identity_name: Option<String>,
        credential_name: Option<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string().into()).collect()),
            credential_exchange_mode,
            timeout: None,
            identity_name: identity_name.map(|x| x.into()),
            credential_name: credential_name.map(|x| x.into()),
        }
    }
}

/// Response body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6056513>,
    #[n(1)] pub addr: Address,
    #[n(2)] pub flow_control_id: FlowControlId
}

impl CreateSecureChannelResponse {
    pub fn new(addr: &Address, flow_control_id: &FlowControlId) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            flow_control_id: flow_control_id.clone(),
        }
    }

    pub fn flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }

    pub fn multiaddr(&self) -> Result<MultiAddr> {
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
    #[b(3)] pub vault: Option<CowStr<'a>>,
    #[b(4)] pub identity: Option<CowStr<'a>>,
}

impl<'a> CreateSecureChannelListenerRequest<'a> {
    pub fn new(
        addr: &Address,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
        vault: Option<String>,
        identity: Option<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string().into()).collect()),
            vault: vault.map(|x| x.into()),
            identity: identity.map(|x| x.into()),
        }
    }
}

/// Request body when deleting a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelListenerRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8293631>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> DeleteSecureChannelListenerRequest<'a> {
    pub fn new(addr: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
        }
    }
}

/// Response body when deleting a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelListenerResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8642885>,
    #[b(1)] pub addr: Option<Cow<'a, str>>,
}

impl<'a> DeleteSecureChannelListenerResponse<'a> {
    pub fn new(addr: Option<Address>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.map(|ch| ch.to_string().into()),
        }
    }
}

/// Request body to show a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelListenerRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3538219>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> ShowSecureChannelListenerRequest<'a> {
    pub fn new(addr: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string().into(),
        }
    }
}

/// Response body to show a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelListenerResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9365445>,
    #[n(1)] pub addr: Address,
    #[n(2)] pub flow_control_id: FlowControlId,
}

impl ShowSecureChannelListenerResponse {
    pub(crate) fn new(info: &SecureChannelListenerInfo) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: info.listener().address().to_string().into(),
            flow_control_id: info.listener().flow_control_id().clone(),
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
    #[b(3)] pub authorized_identifiers: Option<Vec<CowStr<'a>>>,
    #[n(4)] pub flow_control_id: Option<FlowControlId>,
}

impl<'a> ShowSecureChannelResponse<'a> {
    pub fn new(info: Option<&SecureChannelInfo>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: info.map(|info| info.sc().encryptor_address().to_string().into()),
            route: info.map(|info| info.route().to_string().into()),
            authorized_identifiers: info
                .map(|info| {
                    info.authorized_identifiers()
                        .map(|ids| ids.iter().map(|iid| iid.to_string().into()).collect())
                })
                .unwrap_or(None),
            flow_control_id: info.map(|info| info.sc().flow_control_id().clone()),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SecureChannelListenersList {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5141124>,
    #[n(1)] pub list: Vec<ShowSecureChannelListenerResponse>
}

impl SecureChannelListenersList {
    pub fn new(list: Vec<ShowSecureChannelListenerResponse>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
