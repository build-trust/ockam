use minicbor::{CborLen, Decode, Encode};
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddConsumer {
    #[n(1)] flow_control_id: FlowControlId,
    #[n(2)] address: MultiAddr,
}

impl AddConsumer {
    pub fn new(flow_control_id: FlowControlId, address: MultiAddr) -> Self {
        Self {
            flow_control_id,
            address,
        }
    }
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
}
