use minicbor::{Decode, Encode};

use ockam::identity::IdentityIdentifier;
use ockam::remote::RemoteForwarderInfo;
use ockam::route;
use ockam_core::flow_control::FlowControlId;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;
use crate::route_to_multiaddr;

/// Request body when instructing a node to create a forwarder
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateForwarder {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3386455>,
    /// Address to create forwarder at.
    #[n(1)] address: MultiAddr,
    /// Forwarder alias.
    #[b(2)] alias: Option<String>,
    /// Forwarding service is at rust node.
    #[n(3)] at_rust_node: bool,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(4)] authorized: Option<IdentityIdentifier>,
}

impl CreateForwarder {
    pub fn at_project(address: MultiAddr, alias: Option<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            address,
            alias,
            at_rust_node: false,
            authorized: None,
        }
    }

    pub fn at_node(
        address: MultiAddr,
        alias: Option<String>,
        at_rust_node: bool,
        auth: Option<IdentityIdentifier>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
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

    pub fn authorized(&self) -> Option<IdentityIdentifier> {
        self.authorized.clone()
    }
}

/// Response body when creating a forwarder
#[derive(Debug, Clone, Decode, Encode, serde::Serialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ForwarderInfo {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<2757430>,
    #[b(1)] forwarding_route: String,
    #[b(2)] remote_address: String,
    #[b(3)] worker_address: String,
    #[n(4)] flow_control_id: Option<FlowControlId>,
}

impl ForwarderInfo {
    pub fn forwarding_route(&self) -> &str {
        &self.forwarding_route
    }

    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }

    pub fn flow_control_id(&self) -> &Option<FlowControlId> {
        &self.flow_control_id
    }

    pub fn remote_address_ma(&self) -> Result<MultiAddr, ockam_core::Error> {
        route_to_multiaddr(&route![self.remote_address.to_string()])
            .ok_or_else(|| ApiError::generic("Invalid Remote Address"))
    }

    pub fn worker_address_ma(&self) -> Result<MultiAddr, ockam_core::Error> {
        route_to_multiaddr(&route![self.worker_address.to_string()])
            .ok_or_else(|| ApiError::generic("Invalid Worker Address"))
    }
}

impl From<RemoteForwarderInfo> for ForwarderInfo {
    fn from(inner: RemoteForwarderInfo) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            forwarding_route: inner.forwarding_route().to_string(),
            remote_address: inner.remote_address().into(),
            worker_address: inner.worker_address().to_string(),
            flow_control_id: inner.flow_control_id().clone(),
        }
    }
}
