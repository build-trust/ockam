use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HostnamePort {
    pub hostname: String,
    pub port: u16,
}

impl TryInto<ockam_transport_core::HostnamePort> for HostnamePort {
    type Error = ockam_core::Error;
    fn try_into(self) -> Result<ockam_transport_core::HostnamePort, Self::Error> {
        ockam_transport_core::HostnamePort::new(self.hostname, self.port)
    }
}

impl TryFrom<&str> for HostnamePort {
    type Error = ockam_core::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let hostname = ockam_transport_core::HostnamePort::from_str(value)?;
        Ok(HostnamePort {
            hostname: hostname.hostname,
            port: hostname.port,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionStatus {
    Up,
    Down,
}

impl From<crate::ConnectionStatus> for ConnectionStatus {
    fn from(status: crate::ConnectionStatus) -> Self {
        match status {
            crate::ConnectionStatus::Up => ConnectionStatus::Up,
            crate::ConnectionStatus::Down => ConnectionStatus::Down,
        }
    }
}

pub fn default_authority() -> Authority {
    Authority::Project { name: None }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Authority {
    Project {
        /// Name of the project
        /// When omitted, the default project will be used
        name: Option<String>,
    },
    Node {
        /// Multiaddress to the node that will be used as an authority;
        /// When omitted, the default node will be used
        #[schema(example = "/dnsaddr/my-authority.example.com/tcp/4001/secure/api")]
        route: String,
        /// Identifier of the authority node
        #[schema(example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a")]
        identity: String,
        // TODO: Add the possibility to specify the whole public identity
    },
}
