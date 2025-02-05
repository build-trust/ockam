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
    pub tls: OutletTls,
    /// Policy expression that will be used for access control to the TCP Outlet;
    /// by default the policy set for the "tcp-outlet" resource type will be used
    pub allow: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateOutletRequest {
    /// Policy expression that will be used for access control to the TCP Outlet;
    pub allow: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct OutletStatus {
    pub to: HostnamePort,
    pub address: String,
    pub privileged: bool,
}

impl From<crate::nodes::models::portal::OutletStatus> for OutletStatus {
    fn from(status: crate::nodes::models::portal::OutletStatus) -> Self {
        OutletStatus {
            to: HostnamePort {
                hostname: status.to.hostname,
                port: status.to.port,
            },
            address: status.worker_addr.address().to_string(),
            privileged: status.privileged,
        }
    }
}
