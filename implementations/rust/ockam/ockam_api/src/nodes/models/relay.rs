use minicbor::{Decode, Encode};

use ockam::identity::Identifier;
use ockam::remote::RemoteRelayInfo;
use ockam::route;
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;
use crate::route_to_multiaddr;

/// Request body when instructing a node to create a relay
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateRelay {
    /// Address to create relay at.
    #[n(1)] pub(crate) address: MultiAddr,
    /// Relay alias.
    #[n(2)] pub(crate) alias: Option<String>,
    /// Forwarding service is at rust node.
    #[n(3)] pub(crate) at_rust_node: bool,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(4)] pub(crate) authorized: Option<Identifier>,
}

impl CreateRelay {
    pub fn new(
        address: MultiAddr,
        alias: Option<String>,
        at_rust_node: bool,
        auth: Option<Identifier>,
    ) -> Self {
        Self {
            address,
            alias,
            at_rust_node,
            authorized: auth,
        }
    }

    pub fn address(&self) -> &MultiAddr {
        &self.address
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    pub fn at_rust_node(&self) -> bool {
        self.at_rust_node
    }

    pub fn authorized(&self) -> Option<Identifier> {
        self.authorized.clone()
    }
}

/// Response body when creating a relay
#[derive(Debug, Clone, Decode, Encode, serde::Serialize, serde::Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct RelayInfo {
    #[n(1)] forwarding_route: Option<String>,
    #[n(2)] remote_address: Option<String>,
    #[n(3)] worker_address: Option<String>,
    #[n(4)] flow_control_id: Option<FlowControlId>,
    #[n(5)] destination_address: MultiAddr,
    #[n(6)] alias: Option<String>,
    #[n(7)] at_rust_node: bool,
    #[n(8)] key: String,
}

impl RelayInfo {
    pub fn forwarding_route(&self) -> &Option<String> {
        &self.forwarding_route
    }

    pub fn remote_address(&self) -> &Option<String> {
        &self.remote_address
    }

    pub fn flow_control_id(&self) -> &Option<FlowControlId> {
        &self.flow_control_id
    }

    pub fn remote_address_ma(&self) -> Result<Option<MultiAddr>, ockam_core::Error> {
        if let Some(addr) = &self.remote_address {
            route_to_multiaddr(&route![addr.to_string()])
                .ok_or_else(|| ApiError::core("Invalid Remote Address"))
                .map(|ma| Some(ma))
        } else {
            Ok(None)
        }
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn worker_address_ma(&self) -> Result<Option<MultiAddr>, ockam_core::Error> {
        if let Some(addr) = &self.worker_address {
            route_to_multiaddr(&route![addr.to_string()])
                .ok_or_else(|| ApiError::core("Invalid Worker Address"))
                .map(|ma| Some(ma))
        } else {
            Ok(None)
        }
    }

    pub fn with_destination_address(mut self, destination_address: MultiAddr) -> Self {
        self.destination_address = destination_address;
        self
    }

    pub fn with_alias(mut self, alias: Option<String>) -> Self {
        self.alias = alias;
        self
    }

    pub fn with_at_rust_node(mut self, at_rust_node: bool) -> Self {
        self.at_rust_node = at_rust_node;
        self
    }

    pub fn with_key(mut self, key: String) -> Self {
        self.key = key;
        self
    }
}

impl Default for RelayInfo {
    fn default() -> Self {
        Self {
            forwarding_route: None,
            remote_address: None,
            worker_address: None,
            flow_control_id: None,
            destination_address: Default::default(),
            alias: None,
            at_rust_node: false,
            key: "".to_string(),
        }
    }
}

impl From<RemoteRelayInfo> for RelayInfo {
    fn from(inner: RemoteRelayInfo) -> Self {
        Self {
            forwarding_route: Some(inner.forwarding_route().to_string()),
            remote_address: Some(inner.remote_address().into()),
            worker_address: Some(inner.worker_address().to_string()),
            flow_control_id: inner.flow_control_id().clone(),
            destination_address: Default::default(),
            alias: None,
            at_rust_node: false,
            key: "".to_string(),
        }
    }
}
