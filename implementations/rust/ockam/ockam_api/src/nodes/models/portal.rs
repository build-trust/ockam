//! Inlets and outlet request/response types

use colorful::Colorful;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::colors::{color_primary, OckamColor};
use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use ockam::route;
use ockam_abac::Expr;
use ockam_core::{Address, IncomingAccessControl, OutgoingAccessControl, Route};
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::HostnamePort;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::output::Output;
use crate::route_to_multiaddr;
use crate::session::sessions::ConnectionStatus;

/// Request body to create an inlet
#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInlet {
    /// The address the portal should listen at.
    #[n(1)] pub(crate) listen_addr: String,
    /// The peer address.
    /// This can either be the address of an already
    /// created outlet, or a forwarding mechanism via ockam cloud.
    #[n(2)] pub(crate) outlet_addr: MultiAddr,
    /// A human-friendly alias for this portal endpoint
    #[b(3)] pub(crate) alias: String,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(4)] pub(crate) authorized: Option<Identifier>,
    /// A prefix route that will be applied before outlet_addr, and won't be used
    /// to monitor the state of the connection
    #[n(5)] pub(crate) prefix_route: Route,
    /// A suffix route that will be applied after outlet_addr, and won't be used
    /// to monitor the state of the connection
    #[n(6)] pub(crate) suffix_route: Route,
    /// The maximum duration to wait for an outlet to be available
    #[n(7)] pub(crate) wait_for_outlet_duration: Option<Duration>,
    /// The expression for the access control policy for this inlet.
    /// If not set, the policy set for the [TCP inlet resource type](ockam_abac::ResourceType::TcpInlet)
    /// will be used.
    #[n(8)] pub(crate) policy_expression: Option<Expr>,
    /// Create the inlet and wait for the outlet to connect
    #[n(9)] pub(crate) wait_connection: bool,
}

impl CreateInlet {
    pub fn via_project(
        listen: String,
        to: MultiAddr,
        alias: String,
        prefix_route: Route,
        suffix_route: Route,
        wait_connection: bool,
    ) -> Self {
        Self {
            listen_addr: listen,
            outlet_addr: to,
            alias,
            authorized: None,
            prefix_route,
            suffix_route,
            wait_for_outlet_duration: None,
            policy_expression: None,
            wait_connection,
        }
    }

    pub fn to_node(
        listen: String,
        to: MultiAddr,
        alias: String,
        prefix_route: Route,
        suffix_route: Route,
        auth: Option<Identifier>,
        wait_connection: bool,
    ) -> Self {
        Self {
            listen_addr: listen,
            outlet_addr: to,
            alias,
            authorized: auth,
            prefix_route,
            suffix_route,
            wait_for_outlet_duration: None,
            policy_expression: None,
            wait_connection,
        }
    }

    pub fn set_wait_ms(&mut self, ms: u64) {
        self.wait_for_outlet_duration = Some(Duration::from_millis(ms))
    }

    pub fn set_policy_expression(&mut self, expression: Expr) {
        self.policy_expression = Some(expression);
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

    pub fn alias(&self) -> String {
        self.alias.clone()
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
    #[n(1)] pub hostname_port: HostnamePort,
    /// If tls is true a TLS connection is established
    #[n(2)] pub tls: bool,
    /// The address the portal should listen to
    #[n(3)] pub worker_addr: Option<Address>,
    /// Allow the outlet to be reachable from the default secure channel, useful when we want to
    /// tighten the flow control
    #[n(4)] pub reachable_from_default_secure_channel: bool,
    /// The expression for the access control policy for this outlet.
    /// If not set, the policy set for the [TCP outlet resource type](ockam_abac::ResourceType::TcpOutlet)
    /// will be used.
    #[n(5)] pub policy_expression: Option<Expr>,
}

impl CreateOutlet {
    pub fn new(
        hostname_port: HostnamePort,
        tls: bool,
        worker_addr: Option<Address>,
        reachable_from_default_secure_channel: bool,
    ) -> Self {
        Self {
            hostname_port,
            tls,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression: None,
        }
    }

    pub fn set_policy_expression(&mut self, expression: Expr) {
        self.policy_expression = Some(expression);
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode, Serialize, Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletStatus {
    #[n(1)] pub bind_addr: String,
    #[n(2)] pub worker_addr: Option<String>,
    #[n(3)] pub alias: String,
    /// An optional status payload
    #[n(4)] pub payload: Option<String>,
    #[n(5)] pub outlet_route: Option<String>,
    #[n(6)] pub status: ConnectionStatus,
    #[n(7)] pub outlet_addr: String,
}

impl InletStatus {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bind_addr: impl Into<String>,
        worker_addr: impl Into<Option<String>>,
        alias: impl Into<String>,
        payload: impl Into<Option<String>>,
        outlet_route: impl Into<Option<String>>,
        status: ConnectionStatus,
        outlet_addr: impl Into<String>,
    ) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            worker_addr: worker_addr.into(),
            alias: alias.into(),
            payload: payload.into(),
            outlet_route: outlet_route.into(),
            status,
            outlet_addr: outlet_addr.into(),
        }
    }
}

impl Output for InletStatus {
    fn single(&self) -> crate::Result<String> {
        let outlet = self
            .outlet_route
            .as_ref()
            .and_then(Route::parse)
            .and_then(|r| route_to_multiaddr(&r))
            .map(|addr| addr.to_string())
            .unwrap_or("N/A".to_string());

        let output = format!(
            r#"
Inlet
    Alias: {alias}
    Status: {status}
    TCP Address: {bind_addr}
    Outlet Address: {outlet_route}
    Outlet Destination: {outlet_addr}
            "#,
            alias = self
                .alias
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            status = self
                .status
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            bind_addr = self
                .bind_addr
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            outlet_route = outlet.color(OckamColor::PrimaryResource.color()),
            outlet_addr = self.outlet_addr,
        );

        Ok(output)
    }

    fn list(&self) -> crate::Result<String> {
        let output = format!(
            r#"Inlet {}
From {} to {}"#,
            self.alias
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.bind_addr
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.outlet_route
                .as_ref()
                .map(|r| r.to_string())
                .unwrap_or("N/A".to_string())
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode, Serialize, Deserialize, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OutletStatus {
    #[n(1)] pub socket_addr: SocketAddr,
    #[n(2)] pub worker_addr: Address,
    /// An optional status payload
    #[n(3)] pub payload: Option<String>,
}

impl OutletStatus {
    pub fn new(
        socket_addr: SocketAddr,
        worker_addr: Address,
        payload: impl Into<Option<String>>,
    ) -> Self {
        Self {
            socket_addr,
            worker_addr,
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

impl Output for OutletStatus {
    fn single(&self) -> crate::Result<String> {
        let output = format!(
            r#"
Outlet:
    TCP Address:    {}
    Worker Address: {}
"#,
            self.socket_addr,
            self.worker_address()?
        );

        Ok(output)
    }

    fn list(&self) -> crate::Result<String> {
        let output = format!(
            r#"From address {} to TCP server {}"#,
            color_primary(self.worker_address()?.to_string()),
            color_primary(self.socket_addr.to_string()),
        );

        Ok(output)
    }
}

/// Response body when returning a list of Inlets
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletList {
    #[n(1)] pub list: Vec<InletStatus>,
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
    #[n(1)] pub list: Vec<OutletStatus>,
}

impl OutletList {
    pub fn new(list: Vec<OutletStatus>) -> Self {
        Self { list }
    }
}

#[derive(Debug)]
pub enum OutletAccessControl {
    AccessControl(
        (
            Arc<dyn IncomingAccessControl>,
            Arc<dyn OutgoingAccessControl>,
        ),
    ),
    PolicyExpression(Option<Expr>),
}
