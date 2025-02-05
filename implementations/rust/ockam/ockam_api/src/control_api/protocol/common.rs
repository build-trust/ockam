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
