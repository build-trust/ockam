//! Inlets and outlet request/response types

use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use ockam::transport::HostnamePort;
use ockam_abac::PolicyExpression;
use ockam_core::{Address, IncomingAccessControl, OutgoingAccessControl, Route};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};

use crate::colors::color_primary;
use crate::error::ApiError;

use crate::output::Output;
use crate::session::sessions::ConnectionStatus;
use crate::{route_to_multiaddr, try_address_to_multiaddr};

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
    #[n(8)] pub(crate) policy_expression: Option<PolicyExpression>,
    /// Create the inlet and wait for the outlet to connect
    #[n(9)] pub(crate) wait_connection: bool,
    /// The identifier to be used to create the secure channel.
    /// If not set, the node's identifier will be used.
    #[n(10)] pub(crate) secure_channel_identifier: Option<Identifier>,
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
            secure_channel_identifier: None,
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
            secure_channel_identifier: None,
        }
    }

    pub fn set_wait_ms(&mut self, ms: u64) {
        self.wait_for_outlet_duration = Some(Duration::from_millis(ms))
    }

    pub fn set_policy_expression(&mut self, expression: PolicyExpression) {
        self.policy_expression = Some(expression);
    }

    pub fn set_secure_channel_identifier(&mut self, identifier: Identifier) {
        self.secure_channel_identifier = Some(identifier);
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
    #[n(5)] pub policy_expression: Option<PolicyExpression>,
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

    pub fn set_policy_expression(&mut self, expression: PolicyExpression) {
        self.policy_expression = Some(expression);
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Decode, Encode, Serialize)]
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

impl Display for InletStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Inlet at {} is {}",
            color_primary(&self.bind_addr),
            self.status,
        )?;
        if let Some(r) = self
            .outlet_route
            .as_ref()
            .and_then(Route::parse)
            .and_then(|r| route_to_multiaddr(&r))
        {
            write!(f, " with route to outlet {}", color_primary(r.to_string()))?;
        }
        Ok(())
    }
}

impl Output for InletStatus {
    fn item(&self) -> crate::Result<String> {
        Ok(format!("{}", self))
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
        try_address_to_multiaddr(&self.worker_addr)
            .map_err(|_| ApiError::core("Invalid Worker Address"))
    }

    pub fn worker_name(&self) -> Result<String, ockam_core::Error> {
        match self.worker_address()?.last() {
            Some(worker_name) => String::from_utf8(worker_name.data().to_vec())
                .map_err(|_| ApiError::core("Invalid Worker Address")),
            None => Ok(self.worker_addr.to_string()),
        }
    }
}

impl Display for OutletStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outlet at {} is connected to {}",
            color_primary(
                self.worker_address()
                    .map_err(|_| std::fmt::Error)?
                    .to_string()
            ),
            color_primary(self.socket_addr.to_string()),
        )
    }
}

impl Output for OutletStatus {
    fn item(&self) -> Result<String, ApiError> {
        Ok(format!("{}", &self))
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
    WithPolicyExpression(Option<PolicyExpression>),
}
