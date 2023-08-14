use std::time::Duration;

use minicbor::{Decode, Encode};
use serde::Serialize;

use ockam::identity::Identifier;
use ockam_core::flow_control::FlowControlId;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{route, Address, Result};
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;
use crate::nodes::registry::{SecureChannelInfo, SecureChannelListenerInfo};
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
pub struct CreateSecureChannelRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6300395>,
    #[n(1)] pub addr: String,
    #[n(2)] pub authorized_identifiers: Option<Vec<String>>,
    #[n(3)] pub credential_exchange_mode: CredentialExchangeMode,
    #[n(4)] pub timeout: Option<Duration>,
    #[n(5)] pub identity_name: Option<String>,
    #[n(6)] pub credential_name: Option<String>,
}

impl CreateSecureChannelRequest {
    pub fn new(
        addr: &MultiAddr,
        authorized_identifiers: Option<Vec<Identifier>>,
        credential_exchange_mode: CredentialExchangeMode,
        identity_name: Option<String>,
        credential_name: Option<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string()).collect()),
            credential_exchange_mode,
            timeout: None,
            identity_name,
            credential_name,
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
    #[n(2)] pub flow_control_id: FlowControlId,
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
pub struct CreateSecureChannelListenerRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8112242>,
    #[n(1)] pub addr: String,
    #[n(2)] pub authorized_identifiers: Option<Vec<String>>,
    #[n(3)] pub vault: Option<String>,
    #[n(4)] pub identity: Option<String>,
}

impl CreateSecureChannelListenerRequest {
    pub fn new(
        addr: &Address,
        authorized_identifiers: Option<Vec<Identifier>>,
        vault: Option<String>,
        identity: Option<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string(),
            authorized_identifiers: authorized_identifiers
                .map(|x| x.into_iter().map(|y| y.to_string()).collect()),
            vault,
            identity,
        }
    }
}

/// Request body when deleting a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelListenerRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8293631>,
    #[n(1)] pub addr: String,
}

impl DeleteSecureChannelListenerRequest {
    pub fn new(addr: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string(),
        }
    }
}

/// Response body when deleting a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelListenerResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8642885>,
    #[n(1)] pub addr: Address,
}

impl DeleteSecureChannelListenerResponse {
    pub fn new(addr: Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr,
        }
    }
}

/// Request body to show a Secure Channel Listener
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelListenerRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3538219>,
    #[n(1)] pub addr: String,
}

impl ShowSecureChannelListenerRequest {
    pub fn new(addr: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.to_string(),
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
pub struct DeleteSecureChannelRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8472592>,
    #[n(1)] pub channel: String,
}

impl DeleteSecureChannelRequest {
    pub fn new(channel: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: channel.to_string(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6953395>,
    #[n(1)] pub channel: Option<String>,
}

impl DeleteSecureChannelResponse {
    pub fn new(channel: Option<Address>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: channel.map(|ch| ch.to_string()),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3277982>,
    #[n(1)] pub channel: String,
}

impl ShowSecureChannelRequest {
    pub fn new(channel: &Address) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: channel.to_string(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode, Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelResponse {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<4566220>,
    #[n(1)] pub channel: Option<String>,
    #[n(2)] pub route: Option<String>,
    #[n(3)] pub authorized_identifiers: Option<Vec<String>>,
    #[n(4)] pub flow_control_id: Option<FlowControlId>,
}

impl ShowSecureChannelResponse {
    pub fn new(info: Option<&SecureChannelInfo>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            channel: info.map(|info| info.sc().encryptor_address().to_string()),
            route: info.map(|info| info.route().to_string()),
            authorized_identifiers: info
                .map(|info| {
                    info.authorized_identifiers()
                        .map(|ids| ids.iter().map(|iid| iid.to_string()).collect())
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
    #[n(1)] pub list: Vec<ShowSecureChannelListenerResponse>,
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
