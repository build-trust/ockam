use crate::node::Node;
use ockam_core::Address;

#[derive(Clone)]
pub struct Context {
    address: Address,
    node: Node,
}

impl Context {
    pub fn new(node: Node, address: Address) -> Self {
        Self { node, address }
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }

    pub fn node(&self) -> Node {
        self.node.clone()
    }
}
