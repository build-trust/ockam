use crate::common_api::tcp_inlet_create::{parse_to_address, tcp_inlet_default_to_address};
use crate::control_api::protocol::common::{ConnectionStatus, HostPort};
use crate::control_api::ControlApiError;
use crate::CliState;
use ockam::identity::Identifier;
use ockam_abac::PolicyExpression;
use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InletKind {
    /// Uses the provided route to connect to the Outlet
    #[default]
    Regular,
    /// Uses the provided route to connect to the Outlet but
    /// tries to establish a direct UDP communication via UDP puncture
    UdpPuncture,
    /// Only uses a direct UDP communication via UDP puncture
    OnlyUdpPuncture,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    Privileged,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream but
    /// tries to establish a direct UDP communication via UDP puncture
    PrivilegedUdpPuncture,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream but
    /// only uses a direct UDP communication via UDP puncture
    PrivilegedOnlyUdpPuncture,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InletTls {
    ProjectTls,
    CustomTlsProvider {
        /// Route to a certificate provider;
        /// Typical: /project/default/service/tls_certificate_provider
        #[serde(rename = "tls-certificate-provider")]
        tls_certificate_provider: String,
    },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CreateInletRequest {
    /// Name of the TCP Inlet.
    /// When omitted, a random name will be generated.
    #[schema(example = "my-inlet")]
    pub name: Option<String>,

    /// Network address on which to accept TCP connections, in the format `<host>:<port>`.
    /// At least the port must be provided.
    ///
    /// The default host is `127.0.0.1`.
    /// If not set, a random port will be used.
    #[serde(default = "tcp_inlet_default_bind_address")]
    #[schema(default = tcp_inlet_default_bind_address)]
    pub from: HostPort,

    /// Route to a TCP Outlet or the name of the TCP Outlet service you want to connect to.
    ///
    /// If you are connecting to a local node, you can provide the route as `/node/n/service/outlet`.
    ///
    /// If you are connecting to a remote node through a relay in the Orchestrator you can either
    /// provide the full route to the TCP Outlet as `/project/myproject/service/forward_to_myrelay/secure/api/service/outlet`,
    /// or just the service name as `outlet` or `/service/outlet`.
    ///
    /// If you are passing just the service name, consider using `via` to specify the
    /// relay name.
    #[schema(example = "/project/default/service/forward_to_myrelay/secure/api/service/outlet")]
    #[serde(default = "tcp_inlet_default_to_address")]
    pub to: String,

    /// Name of the relay that this TCP Inlet will use to connect to the TCP Outlet.
    ///
    /// Use this flag when you are using `to` to specify the service name of a TCP Outlet
    /// that is reachable through a relay in the Orchestrator.
    ///
    /// If you don't provide it, the default relay name will be used, if necessary.
    #[schema(example = default_via)]
    pub via: Option<String>,

    /// Identity to be used to create the secure channel. If not set, the node's identity will be used.
    #[schema(example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a")]
    pub identity: Option<String>,

    /// Restrict access to the TCP Inlet to the provided identity.
    /// When omitted, all identities are allowed.
    #[schema(example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a")]
    pub authorized: Option<String>,

    /// Policy expression that will be used for access control to the TCP Inlet.
    ///
    /// If you don't provide it, the policy set for the "tcp-inlet" resource type will be used.
    ///
    /// [Learn more about Policy expressions on the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls).
    #[schema(example = "user1")]
    pub allow: Option<String>,

    /// Time to wait in milliseconds before retrying to connect to the TCP Outlet.
    #[serde(default = "retry_wait_default")]
    #[schema(default = retry_wait_default)]
    pub retry_wait: u64,

    /// Kind of the Portal.
    #[serde(default)]
    #[schema(default = InletKind::default)]
    pub kind: InletKind,

    /// TLS Inlet implementation.
    #[serde(default)]
    #[schema(default = default_inlet_tls)]
    pub tls: Option<InletTls>,
}

fn tcp_inlet_default_bind_address() -> HostPort {
    HostPort {
        host: "127.0.0.1".to_string(),
        port: 0,
    }
}

fn retry_wait_default() -> u64 {
    20000
}

fn default_inlet_tls() -> Option<InletTls> {
    None
}

fn default_via() -> Option<String> {
    None
}

pub struct CreateInletRequestValidated {
    pub name: Option<String>,
    pub from: ockam_transport_core::HostnamePort,
    pub to: MultiAddr,
    pub identity: Option<Identifier>,
    pub authorized: Option<Identifier>,
    pub allow: Option<PolicyExpression>,
    pub retry_wait: Duration,
    pub kind: InletKind,
    pub tls: Option<InletTls>,
}

impl CreateInletRequestValidated {
    pub async fn from_request(
        state: &CliState,
        request: CreateInletRequest,
    ) -> Result<Self, ControlApiError> {
        let name = request.name;
        let from = request.from.try_into()?;
        let to = {
            let to = parse_to_address(state, request.to, request.via.as_ref())
                .await
                .map_err(ControlApiError::from)?;
            MultiAddr::try_from(to.as_str()).map_err(ControlApiError::from)?
        };
        let identity = request
            .identity
            .map(|id| Identifier::from_str(&id).map_err(ControlApiError::from))
            .transpose()?;
        let authorized = request
            .authorized
            .map(|id| Identifier::from_str(&id).map_err(ControlApiError::from))
            .transpose()?;
        let allow = request
            .allow
            .map(|allow| PolicyExpression::from_str(&allow).map_err(ControlApiError::from))
            .transpose()?;
        let retry_wait = Duration::from_millis(request.retry_wait);
        let kind = request.kind;
        let tls = request.tls;

        Ok(CreateInletRequestValidated {
            name,
            from,
            to,
            identity,
            authorized,
            allow,
            retry_wait,
            kind,
            tls,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateInletRequest {
    /// Policy expression that will be used for access control to the TCP Inlet.
    ///
    /// If you don't provide it, the policy set for the "tcp-inlet" resource type will be used.
    ///
    /// [Learn more about Policy expressions on the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls).
    #[schema(example = "user1")]
    pub allow: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct InletStatus {
    /// Name of the TCP Inlet
    #[schema(example = "my-inlet")]
    pub name: String,
    /// Status of the TCP Inlet
    #[schema(example = ConnectionStatus::Up)]
    pub status: ConnectionStatus,
    /// Bind address of the TCP Inlet
    #[schema(example = "127.0.0.1:1234")]
    pub bind_address: String,
    /// The current route of the TCP Inlet, populated only when the status is `up`
    // TODO: what shape does this have?
    pub current_route: Option<String>,
    /// Route to the TCP Outlet
    #[schema(example = "/project/default/service/forward_to_myrelay/secure/api/service/outlet")]
    pub to: String,
}

impl TryFrom<crate::nodes::models::portal::InletStatus> for InletStatus {
    type Error = ockam_core::Error;

    fn try_from(status: crate::nodes::models::portal::InletStatus) -> Result<Self, Self::Error> {
        let bind_address = HostPort::try_from(status.bind_addr.as_str())?;
        Ok(InletStatus {
            status: status.status.into(),
            bind_address: bind_address.to_string(),
            name: status.alias,
            current_route: status.outlet_route.map(|r| r.to_string()),
            to: status.outlet_addr,
        })
    }
}
