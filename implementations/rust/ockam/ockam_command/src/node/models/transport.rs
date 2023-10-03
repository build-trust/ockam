use ockam_api::nodes::models::transport::{TransportMode, TransportStatus, TransportType};
use ockam_core::flow_control::FlowControlId;
use serde::Serialize;

/// Information to display of the transports in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowTransportStatus {
    #[serde(rename = "type")]
    pub tt: TransportType,
    pub mode: TransportMode,
    pub socket: String,
    pub worker: String,
    pub flow_control: FlowControlId,
}

impl From<TransportStatus> for ShowTransportStatus {
    fn from(value: TransportStatus) -> Self {
        Self {
            tt: value.tt,
            mode: value.tm,
            socket: value.socket_addr,
            worker: value.worker_addr,
            flow_control: value.flow_control_id,
        }
    }
}
