use serde::Serialize;

use crate::Error;
use ockam_api::{nodes::models::services::ServiceStatus, try_address_to_multiaddr};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

/// Information to display of the services in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowServiceStatus {
    pub address: MultiAddr,
    #[serde(rename = "type")]
    pub service_type: String,
}

impl TryFrom<ServiceStatus> for ShowServiceStatus {
    type Error = Error;

    fn try_from(value: ServiceStatus) -> Result<Self, Self::Error> {
        Ok(Self {
            address: try_address_to_multiaddr(&Address::from_string(&value.addr))?,
            service_type: value.service_type,
        })
    }
}
