use ockam_api::{addr_to_multiaddr, nodes::models::services::ServiceStatus};
use ockam_multiaddr::MultiAddr;
use serde::Serialize;

/// Information to display of the services in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowServiceStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<MultiAddr>,
    #[serde(rename = "type")]
    pub service_type: String,
}

impl From<ServiceStatus> for ShowServiceStatus {
    fn from(value: ServiceStatus) -> Self {
        Self {
            address: addr_to_multiaddr(value.addr),
            service_type: value.service_type,
        }
    }
}
