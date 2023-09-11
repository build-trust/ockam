//! Inlets and outlet request/response types

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use ockam::route;
use ockam_core::compat::borrow::Cow;
use ockam_core::{Address, CowStr, Route};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::route_to_multiaddr;

/// Request body to create an inlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInlet<'a> {
    /// The address the portal should listen at.
    #[n(1)] listen_addr: String,
    /// The peer address.
    /// This can either be the address of an already
    /// created outlet, or a forwarding mechanism via ockam cloud.
    #[n(2)] outlet_addr: MultiAddr,
    /// A human-friendly alias for this portal endpoint
    #[b(3)] alias: Option<CowStr<'a>>,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(4)] authorized: Option<Identifier>,
    /// A prefix route that will be applied before outlet_addr, and won't be used
    /// to monitor the state of the connection
    #[n(5)] prefix_route: Route,
    /// A suffix route that will be applied after outlet_addr, and won't be used
    /// to monitor the state of the connection
    #[n(6)] suffix_route: Route,
    /// The maximum duration to wait for an outlet to be available
    #[n(7)] wait_for_outlet_duration: Option<Duration>,
}

impl<'a> CreateInlet<'a> {
    pub fn via_project(
        listen: String,
        to: MultiAddr,
        prefix_route: Route,
        suffix_route: Route,
    ) -> Self {
        Self {
            listen_addr: listen,
            outlet_addr: to,
            alias: None,
            authorized: None,
            prefix_route,
            suffix_route,
            wait_for_outlet_duration: None,
        }
    }

    pub fn to_node(
        listen: String,
        to: MultiAddr,
        prefix_route: Route,
        suffix_route: Route,
        auth: Option<Identifier>,
    ) -> Self {
        Self {
            listen_addr: listen,
            outlet_addr: to,
            alias: None,
            authorized: auth,
            prefix_route,
            suffix_route,
            wait_for_outlet_duration: None,
        }
    }

    pub fn set_alias(&mut self, a: impl Into<Cow<'a, str>>) {
        self.alias = Some(CowStr(a.into()))
    }

    pub fn set_wait_ms(&mut self, ms: u64) {
        self.wait_for_outlet_duration = Some(Duration::from_millis(ms))
    }

    pub fn listen_addr(&self) -> String {
        self.listen_addr.clone()
    }

    pub fn outlet_addr(&self) -> &MultiAddr {
        &self.outlet_addr
    }

    pub fn authorized(&self) -> Option<Identifier> {
        self.authorized.clone()
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    pub fn prefix_route(&self) -> &Route {
        &self.prefix_route
    }

    pub fn suffix_route(&self) -> &Route {
        &self.suffix_route
    }

    pub fn wait_for_outlet_duration(&self) -> Option<Duration> {
        self.wait_for_outlet_duration
    }
}

/// Request body to create an outlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateOutlet {
    /// The address the portal should connect or bind to
    #[n(1)] pub socket_addr: SocketAddr,
    /// The address the portal should connect or bind to
    #[n(2)] pub worker_addr: Address,
    /// A human-friendly alias for this portal endpoint
    #[n(3)] pub alias: Option<String>,
    /// Allow the outlet to be reachable from the default secure channel, useful when we want to
    /// tighten the flow control
    #[n(4)] pub reachable_from_default_secure_channel: bool,
}

impl CreateOutlet {
    pub fn new(
        socket_addr: SocketAddr,
        worker_addr: Address,
        alias: impl Into<Option<String>>,
        reachable_from_default_secure_channel: bool,
    ) -> Self {
        Self {
            socket_addr,
            worker_addr,
            alias: alias.into(),
            reachable_from_default_secure_channel,
        }
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletStatus {
    #[n(1)] pub bind_addr: String,
    #[n(2)] pub worker_addr: String,
    #[n(3)] pub alias: String,
    /// An optional status payload
    #[n(4)] pub payload: Option<String>,
    #[n(5)] pub outlet_route: String,
}

impl InletStatus {
    pub fn bad_request(reason: &'static str) -> Self {
        Self {
            bind_addr: "".into(),
            worker_addr: "".into(),
            alias: "".into(),
            payload: Some(reason.into()),
            outlet_route: "".into(),
        }
    }

    pub fn new(
        bind_addr: impl Into<String>,
        worker_addr: impl Into<String>,
        alias: impl Into<String>,
        payload: impl Into<Option<String>>,
        outlet_route: impl Into<String>,
    ) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            worker_addr: worker_addr.into(),
            alias: alias.into(),
            payload: payload.into(),
            outlet_route: outlet_route.into(),
        }
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OutletStatus {
    #[n(1)] pub socket_addr: SocketAddr,
    #[n(2)] pub worker_addr: Address,
    #[n(3)] pub alias: String,
    /// An optional status payload
    #[n(4)] pub payload: Option<String>,
}

impl OutletStatus {
    pub fn bad_request(reason: &'static str) -> Self {
        Self {
            socket_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80),
            worker_addr: "".into(),
            alias: "".into(),
            payload: Some(reason.into()),
        }
    }

    pub fn new(
        socket_addr: SocketAddr,
        worker_addr: Address,
        alias: impl Into<String>,
        payload: impl Into<Option<String>>,
    ) -> Self {
        Self {
            socket_addr,
            worker_addr,
            alias: alias.into(),
            payload: payload.into(),
        }
    }

    pub fn worker_address(&self) -> Result<MultiAddr, ockam_core::Error> {
        route_to_multiaddr(&route![self.worker_addr.to_string()])
            .ok_or_else(|| ApiError::core("Invalid Worker Address"))
    }

    pub fn worker_name(&self) -> Result<String, ockam_core::Error> {
        match self.worker_address()?.last() {
            Some(worker_name) => String::from_utf8(worker_name.data().to_vec())
                .map_err(|_| ApiError::core("Invalid Worker Address")),
            None => Ok(self.worker_addr.to_string()),
        }
    }
}

/// Response body when returning a list of Inlets
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletList {
    #[n(1)] pub list: Vec<InletStatus>
}

impl InletList {
    pub fn new(list: Vec<InletStatus>) -> Self {
        Self { list }
    }
}

/// Response body when returning a list of Outlets
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OutletList {
    #[n(1)] pub list: Vec<OutletStatus>
}

impl OutletList {
    pub fn new(list: Vec<OutletStatus>) -> Self {
        Self { list }
    }
}
