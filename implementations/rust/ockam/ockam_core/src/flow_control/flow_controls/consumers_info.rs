use crate::compat::collections::BTreeMap;
use crate::flow_control::FlowControlPolicy;
use crate::Address;

/// Known Consumers for the given [`FlowControlId`]
#[derive(Default, Clone, Debug)]
pub struct ConsumersInfo(pub(super) BTreeMap<Address, FlowControlPolicy>);

impl ConsumersInfo {
    /// Get [`FlowControlPolicy`] for the given [`Address`]
    pub fn get_policy(&self, address: &Address) -> Option<FlowControlPolicy> {
        self.0.get(address).cloned()
    }
}
