use colorful::Colorful;

use std::time::Duration;

use minicbor::{CborLen, Decode, Encode};
use serde::Serialize;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::{Identifier, SecureChannel, SecureChannelListener, DEFAULT_TIMEOUT};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{route, Address, Result};
use ockam_multiaddr::MultiAddr;

use crate::colors::color_primary;
use crate::error::ApiError;
use crate::nodes::registry::SecureChannelInfo;
use crate::output::Output;
use crate::{route_to_multiaddr, try_route_to_multiaddr};

//Requests

/// Request body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelRequest {
    #[n(1)] pub addr: MultiAddr,
    #[n(2)] pub authorized_identifiers: Option<Vec<Identifier>>,
    #[n(4)] pub timeout: Option<Duration>,
    #[n(5)] pub identity_name: Option<String>,
    #[n(6)] pub credential: Option<CredentialAndPurposeKey>,
}

impl CreateSecureChannelRequest {
    pub fn new(
        addr: &MultiAddr,
        authorized_identifiers: Option<Vec<Identifier>>,
        identity_name: Option<String>,
        credential: Option<CredentialAndPurposeKey>,
    ) -> Self {
        Self {
            addr: addr.to_owned(),
            authorized_identifiers,
            timeout: Some(DEFAULT_TIMEOUT),
            identity_name,
            credential,
        }
    }
}

/// Request body when instructing a node to delete a Secure Channel
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelRequest {
    #[n(1)] pub channel: Address,
}

impl DeleteSecureChannelRequest {
    pub fn new(channel: &Address) -> Self {
        Self {
            channel: channel.to_owned(),
        }
    }
}

/// Request body when instructing a node to show a Secure Channel
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelRequest {
    #[n(1)] pub channel: Address,
}

impl ShowSecureChannelRequest {
    pub fn new(channel: &Address) -> Self {
        Self {
            channel: channel.to_owned(),
        }
    }
}

/// Request body when instructing a node to delete a Secure Channel Listener
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelListenerRequest {
    #[n(1)] pub addr: Address,
}

impl DeleteSecureChannelListenerRequest {
    pub fn new(addr: &Address) -> Self {
        Self {
            addr: addr.to_owned(),
        }
    }
}

/// Request body to show a Secure Channel Listener
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelListenerRequest {
    #[n(1)] pub addr: Address,
}

impl ShowSecureChannelListenerRequest {
    pub fn new(addr: &Address) -> Self {
        Self {
            addr: addr.to_owned(),
        }
    }
}

// Responses

/// Response body when instructing a node to create a Secure Channel
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelResponse {
    #[n(1)] pub addr: Address,
    #[n(2)] pub flow_control_id: FlowControlId,
}

impl CreateSecureChannelResponse {
    pub fn new(secure_channel: SecureChannel) -> Self {
        Self {
            addr: secure_channel.encryptor_address().to_string().into(),
            flow_control_id: secure_channel.flow_control_id().clone(),
        }
    }

    pub fn flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        route_to_multiaddr(&route![self.addr.to_string()])
            .ok_or_else(|| ApiError::core(format!("Invalid route: {}", self.addr)))
    }
}

impl Output for CreateSecureChannelResponse {
    fn item(&self) -> crate::Result<String> {
        let addr = try_route_to_multiaddr(&route![self.addr.to_string()])?.to_string();
        Ok(addr)
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecureChannelListenerRequest {
    #[n(1)] pub addr: Address,
    #[n(2)] pub authorized_identifiers: Option<Vec<Identifier>>,
    #[n(3)] pub identity_name: Option<String>,
}

impl CreateSecureChannelListenerRequest {
    pub fn new(
        addr: &Address,
        authorized_identifiers: Option<Vec<Identifier>>,
        identity_name: Option<String>,
    ) -> Self {
        Self {
            addr: addr.to_owned(),
            authorized_identifiers,
            identity_name,
        }
    }
}

/// Response body when deleting a Secure Channel Listener
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelListenerResponse {
    #[n(1)] pub addr: Address,
}

impl DeleteSecureChannelListenerResponse {
    pub fn new(addr: Address) -> Self {
        Self { addr }
    }
}

impl Output for SecureChannelListener {
    fn item(&self) -> crate::Result<String> {
        let addr = {
            let channel_route = route![self.address().clone()];
            let channel_multiaddr = try_route_to_multiaddr(&channel_route)?;
            channel_multiaddr.to_string()
        };
        Ok(format!("Listener at {}", color_primary(addr)))
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteSecureChannelResponse {
    #[n(1)] pub channel: Option<String>,
}

impl DeleteSecureChannelResponse {
    pub fn new(channel: Option<Address>) -> Self {
        Self {
            channel: channel.map(|ch| ch.to_string()),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen, Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ShowSecureChannelResponse {
    #[n(1)] pub channel: Option<String>,
    #[n(2)] pub route: Option<String>,
    #[n(3)] pub authorized_identifiers: Option<Vec<String>>,
    #[n(4)] pub flow_control_id: Option<FlowControlId>,
}

impl ShowSecureChannelResponse {
    pub fn new(info: Option<SecureChannelInfo>) -> Self {
        Self {
            channel: info
                .clone()
                .map(|info| info.sc().encryptor_address().to_string()),
            route: info.clone().map(|info| info.route().to_string()),
            authorized_identifiers: info
                .clone()
                .map(|info| {
                    info.clone()
                        .authorized_identifiers()
                        .map(|ids| ids.iter().map(|iid| iid.to_string()).collect())
                })
                .unwrap_or(None),
            flow_control_id: info.map(|info| info.sc().flow_control_id().clone()),
        }
    }
}

impl Output for ShowSecureChannelResponse {
    fn item(&self) -> crate::Result<String> {
        let s = match &self.channel {
            Some(addr) => {
                format!(
                    "\n  Secure Channel:\n{} {}\n{} {}\n{} {}",
                    "  •         At: ".light_magenta(),
                    try_route_to_multiaddr(&route![addr.to_string()])?
                        .to_string()
                        .light_yellow(),
                    "  •         To: ".light_magenta(),
                    self.route.clone().unwrap().light_yellow(),
                    "  • Authorized: ".light_magenta(),
                    self.authorized_identifiers
                        .as_ref()
                        .unwrap_or(&vec!["none".to_string()])
                        .iter()
                        .map(|id| id.clone().light_yellow().to_string())
                        .collect::<Vec<String>>()
                        .join("\n\t")
                )
            }
            None => format!("{}", "Channel not found".red()),
        };

        Ok(s)
    }
}
