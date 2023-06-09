use crate::compat::collections::{BTreeMap, BTreeSet};
use crate::flow_control::SpawnerFlowControlPolicy;
use crate::Address;

/// Known Consumers for the given [`FlowControlId`]
#[derive(Default, Clone, Debug)]
pub struct SpawnerConsumersInfo(pub(super) BTreeMap<Address, SpawnerFlowControlPolicy>);

impl SpawnerConsumersInfo {
    /// Get [`FlowControlPolicy`] for the given [`Address`]
    pub fn get_policy(&self, address: &Address) -> Option<SpawnerFlowControlPolicy> {
        self.0.get(address).cloned()
    }
}

/// Known Consumers for the given [`FlowControlId`]
#[derive(Default, Clone, Debug)]
pub struct ProducerConsumersInfo(pub(super) BTreeSet<Address>);

impl ProducerConsumersInfo {
    /// Get [`FlowControlPolicy`] for the given [`Address`]
    pub fn contains(&self, address: &Address) -> bool {
        self.0.contains(address)
    }
}
