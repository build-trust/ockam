use minicbor::{Decode, Encode};
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddConsumer {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2016565>,
    #[n(1)] flow_control_id: FlowControlId,
    #[n(2)] address: MultiAddr,
    #[n(3)] policy: FlowControlPolicy,
}

impl AddConsumer {
    pub fn new(
        flow_control_id: FlowControlId,
        address: MultiAddr,
        policy: FlowControlPolicy,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            flow_control_id,
            address,
            policy,
        }
    }
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
    pub fn policy(&self) -> FlowControlPolicy {
        self.policy
    }
}
