use serde::Serialize;

use ockam_api::nodes::models::transport::{TransportMode, TransportStatus, TransportType};

/// Information to display of the transports in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowTransportStatus {
    #[serde(rename = "type")]
    pub tt: TransportType,
    pub mode: TransportMode,
    pub socket: String,
}

impl From<TransportStatus> for ShowTransportStatus {
    fn from(value: TransportStatus) -> Self {
        Self {
            tt: value.tt,
            mode: value.tm,
            socket: value.socket_addr,
        }
    }
}
