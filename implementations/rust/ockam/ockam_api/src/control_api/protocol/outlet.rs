use crate::control_api::protocol::common::HostnamePort;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OutletKind {
    /// Works as a regular TCP Outlet. It's compatible with UDP Puncture,
    /// but it must be enabled at node level.
    #[default]
    Regular,
    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    /// It's compatible with UDP Puncture, but it must be enabled at node level.
    Privileged,
}

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OutletTls {
    #[default]
    /// No TLS
    None,
    /// The destination uses TLS, the connection will be fully validated.
    Validate,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CreateOutletRequest {
    /// The kind of the outlet
    pub kind: OutletKind,
    /// The address of the outlet, also acts as an identifier for the resource
    pub address: Option<String>,
    /// The destination address of the TCP connection
    pub to: HostnamePort,
    /// The TLS configuration for the outlet
    #[serde(default)]
    #[schema(default = "None")]
    pub tls: OutletTls,
    /// Policy expression that will be used for access control to the TCP Outlet;
    /// by default, the policy set for the "tcp-outlet" resource type will be used.
    /// [Learn more about Policies expression on the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls).
    pub allow: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateOutletRequest {
    /// Policy expression that will be used for access control to the TCP Outlet;
    /// by default, the policy set for the "tcp-outlet" resource type will be used.
    /// [Learn more about Policies expression on the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls).
    pub allow: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct OutletStatus {
    /// The address of the outlet
    pub to: HostnamePort,
    /// The address of the worker, this also acts as an identifier of the TCP Outlet within the node
    pub address: String,
    /// Whether the outlet is of privileged kind
    pub privileged: bool,
}

impl From<crate::nodes::models::portal::TcpOutletInfo> for OutletStatus {
    fn from(status: crate::nodes::models::portal::TcpOutletInfo) -> Self {
        OutletStatus {
            to: HostnamePort {
                hostname: status.parameters.to.hostname,
                port: status.parameters.to.port,
            },
            address: status.worker_address.address().to_string(),
            privileged: status.parameters.privileged,
        }
    }
}
