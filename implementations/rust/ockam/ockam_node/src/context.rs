use super::Address;
use super::Node;

/// Execution context. Meta-information and [`Node`] API references.
#[derive(Clone, Debug)]
pub struct Context {
    /// [`Address`] of this Context.
    pub address: Address,
    /// The Ockam [`Node`] API.
    pub node: Node,
}

impl Context {
    /// Create a new [`Context`] on the [`Node`], registered at [`Address`].
    pub fn new(node: Node, address: Address) -> Self {
        Self { node, address }
    }
}
