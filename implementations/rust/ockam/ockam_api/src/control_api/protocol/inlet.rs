use crate::control_api::protocol::common::HostnamePort;
use serde::{Deserialize, Serialize};

fn tcp_inlet_default_bind_address() -> HostnamePort {
    HostnamePort {
        hostname: "127.0.0.1".to_string(),
        port: 0,
    }
}

fn retry_wait_default() -> u64 {
    20000
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub(in crate::control_api) enum InletKind {
    /// Uses the provided Multiaddress to connect to the Outlet
    #[default]
    Regular,
    /// Uses the provided Multiaddress to connect to the Outlet but
    /// tries to establish a direct UDP communication via UDP puncture
    UdpPucture,
    /// Only uses a direct UDP communication via UDP puncture
    OnlyUdpPucture,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    Privileged,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream but
    /// tries to establish a direct UDP communication via UDP puncture
    PrivilegedUdpPuncture,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream but
    /// only uses a direct UDP communication via UDP puncture
    PrivilegedOnlyUdpPuncture,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub(in crate::control_api) enum InletTls {
    #[default]
    None,
    ProjectTls,
    CustomTlsProvider {
        /// Multiaddress to a certificate provider;
        /// Typical: /project/default/service/tls_certificate_provider
        #[serde(rename = "tls-certificate-provider")]
        tls_certificate_provider: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(in crate::control_api) enum ConnectionStatus {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::control_api) struct CreateInletRequest {
    /// Name of the TCP Inlet;
    /// By default, a random name will be generated
    pub name: Option<String>,
    /// Kind of the Portal
    #[serde(flatten)]
    pub kind: InletKind,
    /// TLS Inlet implementation
    #[serde(default)]
    pub tls: InletTls,
    /// Bind address for the TCP Inlet
    #[serde(default = "tcp_inlet_default_bind_address")]
    pub from: HostnamePort,
    /// Multiaddress to a TCP Outlet
    /// Example: /project/default/service/forward_to_node1/secure/api/service/outlet
    pub to: String,
    /// Identity to be used to create the secure channel;
    /// by default, the node's identity will be used
    pub identity: Option<String>,
    /// Restrict access to the TCP Inlet to the provided identity;
    /// by default, all identities are allowed;
    /// Example: "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"
    pub authorized: Option<String>,
    /// Policy expression that will be used for access control to the TCP Inlet;
    /// by default the policy set for the "tcp-inlet" resource type will be used
    pub allow: Option<String>,
    /// When connection is lost, how long to wait before retrying to connect to the TCP Outlet;
    /// In milliseconds;
    #[serde(default = "retry_wait_default")]
    pub retry_wait: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(in crate::control_api) struct InletStatus {
    pub name: String,
    pub status: ConnectionStatus,
    pub bind_address: HostnamePort,
    pub current_route: Option<String>,
    pub to: String,
    pub privileged: bool,
}

impl TryFrom<crate::nodes::models::portal::InletStatus> for InletStatus {
    type Error = ockam_core::Error;

    fn try_from(status: crate::nodes::models::portal::InletStatus) -> Result<Self, Self::Error> {
        let bind_address = HostnamePort::try_from(status.bind_addr.as_str())?;

        Ok(InletStatus {
            status: match status.status {
                crate::ConnectionStatus::Up => ConnectionStatus::Up,
                crate::ConnectionStatus::Down => ConnectionStatus::Down,
            },
            bind_address,
            name: status.alias,
            current_route: status.outlet_route.map(|r| r.to_string()),
            to: status.outlet_addr,
            privileged: status.privileged,
        })
    }
}
