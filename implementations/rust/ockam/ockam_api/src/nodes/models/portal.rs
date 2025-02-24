//! Inlets and outlet request/response types

use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::colors::{color_primary, color_primary_alt};
use crate::error::ApiError;
use crate::multiaddr_resolver::{
    deserialize_address_from_local_service, serialize_address_as_local_service,
};
use crate::nodes::registry::InletStateSummary;
use crate::output::Output;
use crate::session::connection_status::ConnectionStatus;
use crate::terminal::fmt;
use crate::ReverseLocalConverter;
use minicbor::{CborLen, Decode, Encode};
use ockam::identity::Identifier;
use ockam::transport::HostnamePort;
use ockam::Message;
use ockam_abac::PolicyExpression;
use ockam_core::{
    cbor_encode_preallocate, Address, Decodable, Encodable, Encoded, IncomingAccessControl,
    OutgoingAccessControl, Route,
};
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};

/// Request body to create an inlet
#[derive(Clone, Debug, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInlet {
    /// The address the portal should listen at.
    #[n(1)] pub(crate) listen_addr: HostnamePort,
    /// The peer address.
    /// This can either be the address of an already
    /// created outlet, or a forwarding mechanism via ockam cloud.
    #[n(2)] pub(crate) outlet_addresses: Vec<MultiAddr>,
    /// TCP Inlet route redundancy. 0 means only one route is instantiated.
    #[n(3)] pub(crate) target_redundancy: usize,
    /// A human-friendly alias for this portal endpoint
    #[b(4)] pub(crate) alias: String,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(5)] pub(crate) authorized: Option<Identifier>,
    /// How long until the outlet route is considered disconnected.
    #[n(6)] pub(crate) ping_timeout: Option<Duration>,
    /// The maximum duration to wait for an outlet to be available
    #[n(7)] pub(crate) wait_for_outlet: Option<Duration>,
    /// The expression for the access control policy for this inlet.
    /// If not set, the policy set for the [TCP inlet resource type](ockam_abac::ResourceType::TcpInlet)
    /// will be used.
    #[n(8)] pub(crate) policy_expression: Option<PolicyExpression>,
    /// Create the inlet and wait for the outlet to connect
    #[n(9)] pub(crate) wait_connection: bool,
    /// The identifier to be used to create the secure channel.
    /// If not set, the node's identifier will be used.
    #[n(10)] pub(crate) secure_channel_identifier: Option<Identifier>,
    /// Enable UDP NAT puncture.
    #[n(11)] pub(crate) enable_udp_puncture: bool,
    /// Disable fallback to TCP.
    /// TCP won't be used to transfer data between the Inlet and the Outlet.
    #[n(12)] pub(crate) disable_tcp_fallback: bool,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    #[n(13)] pub(crate) privileged: bool,
    /// TLS certificate provider route.
    #[n(14)] pub(crate) tls_certificate_provider: Option<MultiAddr>,
    /// Skip Portal handshake for lower latency, but also lower throughput
    #[n(15)] pub(crate) skip_handshake: bool,
    /// Enable Nagle's algorithm for potentially higher throughput, but higher latency
    #[n(16)] pub(crate) enable_nagle: bool,
    /// The prefix route to be used for interceptors.
    #[n(17)] pub(crate) prefix_route: Route,
}

impl Encodable for CreateInlet {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateInlet {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateInlet {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        listen: HostnamePort,
        target_redundancy: usize,
        to: Vec<MultiAddr>,
        alias: String,
        auth: Option<Identifier>,
        wait_connection: bool,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
        privileged: bool,
        skip_handshake: bool,
        enable_nagle: bool,
    ) -> Self {
        Self {
            listen_addr: listen,
            outlet_addresses: to,
            target_redundancy,
            alias,
            authorized: auth,
            ping_timeout: None,
            wait_for_outlet: None,
            policy_expression: None,
            wait_connection,
            secure_channel_identifier: None,
            enable_udp_puncture,
            disable_tcp_fallback,
            privileged,
            tls_certificate_provider: None,
            skip_handshake,
            enable_nagle,
            prefix_route: Default::default(),
        }
    }

    pub fn set_tls_certificate_provider(&mut self, provider: MultiAddr) {
        self.tls_certificate_provider = Some(provider);
    }

    pub fn set_prefix_route(&mut self, route: Route) {
        self.prefix_route = route;
    }

    pub fn set_ping_timeout(&mut self, ping_timeout: Duration) {
        self.ping_timeout = Some(ping_timeout)
    }

    pub fn set_wait_for_outlet(&mut self, wait_for_outlet_duration: Duration) {
        self.wait_for_outlet = Some(wait_for_outlet_duration);
    }

    pub fn set_policy_expression(&mut self, expression: PolicyExpression) {
        self.policy_expression = Some(expression);
    }

    pub fn set_secure_channel_identifier(&mut self, identifier: Identifier) {
        self.secure_channel_identifier = Some(identifier);
    }
}

/// Request body to create an outlet
#[derive(Clone, Debug, Encode, Decode, CborLen, Message)]
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
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    #[n(6)] pub privileged: bool,
    /// Skip Portal handshake for lower latency, but also lower throughput
    #[n(7)] pub skip_handshake: bool,
    /// Enable Nagle's algorithm for potentially higher throughput, but higher latency
    #[n(8)] pub(crate) enable_nagle: bool,
}

impl Encodable for CreateOutlet {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for CreateOutlet {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl CreateOutlet {
    pub fn new(
        hostname_port: HostnamePort,
        tls: bool,
        worker_addr: Option<Address>,
        reachable_from_default_secure_channel: bool,
        privileged: bool,
        skip_handshake: bool,
        enable_nagle: bool,
    ) -> Self {
        Self {
            hostname_port,
            tls,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression: None,
            privileged,
            skip_handshake,
            enable_nagle,
        }
    }

    pub fn set_policy_expression(&mut self, expression: PolicyExpression) {
        self.policy_expression = Some(expression);
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InletStatusView {
    #[n(1)] pub bind_address: String,
    #[n(2)] pub alias: String,
    #[n(3)] pub outlet_addresses: Vec<String>,
    #[n(4)] pub privileged: bool,
    /// Status of the inlet, up if at least one session is up
    #[n(5)] pub connection: ConnectionStatus,
    #[n(6)] pub outlet_routes: Vec<String>,
    #[n(7)] pub target_redundancy: usize,
}

impl Encodable for InletStatusView {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for InletStatusView {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl InletStatusView {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        bind_address: SocketAddr,
        alias: impl Into<String>,
        outlet_addresses: &[MultiAddr],
        privileged: bool,
        summary: InletStateSummary,
    ) -> Self {
        Self {
            bind_address: bind_address.to_string(),
            alias: alias.into(),
            outlet_routes: summary
                .active_routes
                .iter()
                .map(|s| s.route.to_string())
                .collect(),
            target_redundancy: summary.target_redundancy,
            connection: summary.connection_status(),
            outlet_addresses: outlet_addresses.iter().map(|a| a.to_string()).collect(),
            privileged,
        }
    }
}

impl Display for InletStatusView {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "TCP Inlet {} at {} is {}",
            color_primary(&self.alias),
            color_primary(&self.bind_address),
            self.connection,
        )?;

        if self.outlet_addresses.is_empty() {
            writeln!(f, "{}With no outlet address", fmt::INDENTATION)?;
        } else {
            writeln!(f, "{}With outlet address:", fmt::INDENTATION)?;
            for address in &self.outlet_addresses {
                writeln!(
                    f,
                    "{}{}{}",
                    fmt::INDENTATION,
                    fmt::INDENTATION,
                    color_primary(address)
                )?;
            }
        }

        if self.outlet_routes.is_empty() {
            writeln!(f, "{}With no service routes", fmt::INDENTATION)?;
        } else {
            writeln!(f, "{}With service routes:", fmt::INDENTATION)?;
            for route in self
                .outlet_routes
                .iter()
                .flat_map(Route::parse)
                .flat_map(|r| ReverseLocalConverter::convert_route(&r).ok())
            {
                writeln!(
                    f,
                    "{}{}{}",
                    fmt::INDENTATION,
                    fmt::INDENTATION,
                    color_primary(route.to_string())
                )?;
            }
        }

        if self.privileged {
            writeln!(
                f,
                "{}Operating in {} mode",
                fmt::INDENTATION,
                color_primary_alt("privileged".to_string())
            )?;
        }
        Ok(())
    }
}

impl Output for InletStatusView {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}

#[derive(Encode, Decode, CborLen, Debug, Default, Clone, Message)]
pub struct InletStatusList(#[n(0)] pub Vec<InletStatusView>);

impl Encodable for InletStatusList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for InletStatusList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

/// Response body when interacting with a portal endpoint
#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize, Deserialize, PartialEq, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OutletStatus {
    #[n(1)] pub to: HostnamePort,
    #[serde(serialize_with = "serialize_address_as_local_service")]
    #[serde(deserialize_with = "deserialize_address_from_local_service")]
    #[n(2)] pub worker_address: Address,
    /// An optional status payload
    #[n(3)] pub payload: Option<String>,
    #[n(4)] pub privileged: bool,
}

impl Encodable for OutletStatus {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for OutletStatus {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl OutletStatus {
    pub fn new(
        to: HostnamePort,
        worker_address: Address,
        payload: impl Into<Option<String>>,
        privileged: bool,
    ) -> Self {
        Self {
            to,
            worker_address,
            payload: payload.into(),
            privileged,
        }
    }

    pub fn worker_route(&self) -> Result<MultiAddr, ockam_core::Error> {
        ReverseLocalConverter::convert_address(&self.worker_address)
    }

    pub fn worker_name(&self) -> Result<String, ockam_core::Error> {
        match self.worker_route()?.last() {
            Some(worker_name) => String::from_utf8(worker_name.data().to_vec())
                .map_err(|_| ApiError::core("Invalid Worker Address")),
            None => Ok(self.worker_address.to_string()),
        }
    }
}

impl Display for OutletStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TCP Outlet at {} is connected to {}",
            color_primary(
                self.worker_route()
                    .map_err(|_| std::fmt::Error)?
                    .to_string()
            ),
            color_primary(self.to.to_string()),
        )?;

        if self.privileged {
            writeln!(
                f,
                "{}Operating in {} mode",
                fmt::INDENTATION,
                color_primary_alt("privileged".to_string())
            )?;
        }

        Ok(())
    }
}

#[derive(Encode, Decode, CborLen, Debug, Default, Clone, Message)]
#[cbor(transparent)]
pub struct OutletStatusList(#[n(0)] pub Vec<OutletStatus>);

impl Encodable for OutletStatusList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for OutletStatusList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}

impl Output for OutletStatus {
    fn item(&self) -> Result<String, ApiError> {
        Ok(self.padded_display())
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
