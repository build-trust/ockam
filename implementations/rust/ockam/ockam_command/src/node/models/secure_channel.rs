use ockam_api::{
    addr_to_multiaddr, nodes::models::secure_channel::ShowSecureChannelListenerResponse,
};
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;
use serde::Serialize;

/// Information to display of the secure channel listeners in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowSecureChannelListener {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<MultiAddr>,
    pub flow_control: FlowControlId,
}

impl From<ShowSecureChannelListenerResponse> for ShowSecureChannelListener {
    fn from(value: ShowSecureChannelListenerResponse) -> Self {
        Self {
            address: addr_to_multiaddr(value.addr),
            flow_control: value.flow_control_id,
        }
    }
}
