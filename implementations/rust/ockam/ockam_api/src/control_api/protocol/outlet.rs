use crate::control_api::protocol::common::{HostPortRequest, HostPortResponse};
use crate::control_api::ControlApiError;
use ockam_abac::PolicyExpression;
use ockam_core::Address;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, EnumString, Default, ToSchema)]
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

#[derive(Debug, Serialize, Deserialize, EnumString, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OutletTls {
    /// The destination is expected to be a TLS endpoint and will be fully validated.
    Validate,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CreateOutletRequest {
    /// Service address of your TCP Outlet.
    ///
    /// This unique address identifies the TCP Outlet worker on the Node on your local machine.
    /// Examples are `/service/my-outlet` or `my-outlet`.
    ///
    /// If not provided, `outlet` will be used, or a random address will be generated if `outlet` is taken.
    /// You will need this address when creating a TCP Inlet.
    #[serde(alias = "address")]
    #[schema(example = "my-outlet")]
    pub name: Option<String>,

    /// Network address where your application is listening to, in the format `<host>:<port>`.
    /// Your TCP Outlet will forward raw TCP traffic to this destination.
    #[schema(example = "dev.environment:1234")]
    pub to: HostPortRequest,

    /// The TLS configuration for the TCP Outlet.
    #[serde(default)]
    #[schema(default = default_outlet_tls)]
    pub tls: Option<OutletTls>,

    /// Policy expression that will be used for access control to the TCP Outlet.
    ///
    /// If you don't provide it, the policy set for the "tcp-outlet" resource type will be used.
    ///
    /// [Learn more about Policy expressions on the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls).
    #[schema(example = "user1")]
    pub allow: Option<String>,

    /// The kind of the TCP Outlet.
    #[serde(default)]
    #[schema(default = OutletKind::default)]
    pub kind: OutletKind,
}

fn default_outlet_tls() -> Option<OutletTls> {
    None
}

pub struct CreateOutletRequestValidated {
    pub name: Option<Address>,
    pub to: ockam_transport_core::HostnamePort,
    pub tls: Option<OutletTls>,
    pub allow: Option<PolicyExpression>,
    pub kind: OutletKind,
}

impl TryFrom<CreateOutletRequest> for CreateOutletRequestValidated {
    type Error = ControlApiError;

    fn try_from(request: CreateOutletRequest) -> Result<Self, Self::Error> {
        let name = match request.name {
            Some(name) => Some(Address::from_str(&name).map_err(crate::error::ParseError::from)?),
            None => None,
        };
        let to = request.to.try_into()?;
        let allow = match &request.allow {
            Some(allow) => Some(PolicyExpression::from_str(allow).map_err(ControlApiError::from)?),
            None => None,
        };
        Ok(CreateOutletRequestValidated {
            name,
            to,
            tls: request.tls,
            allow,
            kind: request.kind,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateOutletRequest {
    /// Policy expression that will be used for access control to the TCP Outlet.
    ///
    /// If you don't provide it, the policy set for the "tcp-outlet" resource type will be used.
    ///
    /// [Learn more about Policy expressions on the Ockam documentation](https://docs.ockam.io/reference/protocols/access-controls).
    #[schema(example = "user1")]
    pub allow: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct OutletStatus {
    /// Network address of the TCP Outlet, in the format `<host>:<port>`.
    pub to: HostPortResponse,
    /// Name, or service address, of the TCP Outlet.
    /// It acts as the identifier of the TCP Outlet within the node.
    #[schema(example = "my-outlet", deprecated)]
    pub address: String,
    /// Name, or service address, of the TCP Outlet.
    /// It acts as the identifier of the TCP Outlet within the node.
    #[schema(example = "my-outlet")]
    pub name: String,
    /// Whether the TCP Outlet is of privileged kind.
    pub privileged: bool,
}

impl TryFrom<crate::nodes::models::portal::OutletStatus> for OutletStatus {
    type Error = ockam_core::Error;

    fn try_from(status: crate::nodes::models::portal::OutletStatus) -> Result<Self, Self::Error> {
        let to = HostPortResponse {
            host: status.to.hostname,
            port: status.to.port,
        };
        Ok(OutletStatus {
            to,
            address: status.worker_address.address().to_string(),
            name: status.worker_address.address().to_string(),
            privileged: status.privileged,
        })
    }
}
