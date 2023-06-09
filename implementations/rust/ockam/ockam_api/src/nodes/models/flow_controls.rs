use minicbor::{Decode, Encode};
use ockam_core::flow_control::{
    ProducerFlowControlId, SpawnerFlowControlId, SpawnerFlowControlPolicy,
};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddConsumerForProducer {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2016565>,
    #[n(1)] flow_control_id: ProducerFlowControlId,
    #[n(2)] address: MultiAddr,
}

impl AddConsumerForProducer {
    pub fn new(flow_control_id: ProducerFlowControlId, address: MultiAddr) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            flow_control_id,
            address,
        }
    }
    pub fn flow_control_id(&self) -> &ProducerFlowControlId {
        &self.flow_control_id
    }
    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddConsumerForSpawner {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1892360>,
    #[n(1)] flow_control_id: SpawnerFlowControlId,
    #[n(2)] address: MultiAddr,
    #[n(3)] policy: SpawnerFlowControlPolicy,
}

impl AddConsumerForSpawner {
    pub fn new(
        flow_control_id: SpawnerFlowControlId,
        address: MultiAddr,
        policy: SpawnerFlowControlPolicy,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            flow_control_id,
            address,
            policy,
        }
    }
    pub fn flow_control_id(&self) -> &SpawnerFlowControlId {
        &self.flow_control_id
    }
    pub fn address(&self) -> &MultiAddr {
        &self.address
    }
    pub fn policy(&self) -> SpawnerFlowControlPolicy {
        self.policy
    }
}
