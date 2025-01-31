use minicbor::{CborLen, Decode, Encode};
use ockam::Message;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{cbor_encode_preallocate, Decodable, Encodable, Encoded};
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddConsumer {
    #[n(1)] flow_control_id: FlowControlId,
    #[n(2)] address: MultiAddr,
}

impl Encodable for AddConsumer {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for AddConsumer {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
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
