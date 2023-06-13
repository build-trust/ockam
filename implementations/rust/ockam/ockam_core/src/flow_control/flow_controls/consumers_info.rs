use crate::compat::collections::BTreeSet;
use crate::Address;

/// Known Consumers for the given [`FlowControlId`]
#[derive(Default, Clone, Debug)]
pub struct ConsumersInfo(pub(super) BTreeSet<Address>);

impl ConsumersInfo {
    /// Get [`FlowControlPolicy`] for the given [`Address`]
    pub fn contains(&self, address: &Address) -> bool {
        self.0.contains(address)
    }
}
