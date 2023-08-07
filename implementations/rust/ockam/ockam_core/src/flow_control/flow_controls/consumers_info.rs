use crate::compat::collections::BTreeSet;
use crate::Address;

/// Known Consumers for the given [`FlowControlId`]
#[derive(Default, Clone, Debug)]
pub struct ConsumersInfo(pub(super) BTreeSet<Address>);

impl ConsumersInfo {
    /// Check if given [`Address`] is a consumer
    pub fn contains(&self, address: &Address) -> bool {
        self.0.contains(address)
    }
}
