use super::Address;
use super::Node;

#[derive(Clone, Debug)]
pub struct Context<T> {
    pub address: Address,
    pub node: Node<T>,
}

impl<T> Context<T> {
    pub fn new(node: Node<T>, address: Address) -> Self {
        Self { node, address }
    }
}
