use minicbor::{Decode, Encode};

use ockam::identity::IdentityIdentifier;
use ockam::remote::RemoteForwarderInfo;
use ockam_core::CowStr;
use ockam_multiaddr::MultiAddr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body when instructing a node to create a forwarder
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateForwarder<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3386455>,
    /// Address to create forwarder at.
    #[n(1)] address: MultiAddr,
    /// Forwarder alias.
    #[b(2)] alias: Option<CowStr<'a>>,
    /// Forwarding service is at rust node.
    #[n(3)] at_rust_node: bool,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(4)] authorized: Option<IdentityIdentifier>
}

impl<'a> CreateForwarder<'a> {
    pub fn at_project(address: MultiAddr, alias: Option<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            address,
            alias: alias.map(|s| s.into()),
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
            alias: alias.map(|s| s.into()),
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
pub struct ForwarderInfo<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] tag: TypeTag<2757430>,
    #[b(1)] forwarding_route: CowStr<'a>,
    #[b(2)] remote_address: CowStr<'a>,
    #[b(3)] worker_address: CowStr<'a>,
}

impl<'a> ForwarderInfo<'a> {
    pub fn forwarding_route(&'a self) -> &'a str {
        &self.forwarding_route
    }

    pub fn remote_address(&'a self) -> &'a str {
        &self.remote_address
    }

    pub fn worker_address(&'a self) -> &'a str {
        &self.worker_address
    }
}

impl<'a> From<RemoteForwarderInfo> for ForwarderInfo<'a> {
    fn from(inner: RemoteForwarderInfo) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            forwarding_route: inner.forwarding_route().to_string().into(),
            remote_address: inner.remote_address().to_string().into(),
            worker_address: inner.worker_address().to_string().into(),
        }
    }
}
