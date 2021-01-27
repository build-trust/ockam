use super::Node;

#[derive(Clone, Debug)]
pub struct Context {
    address: String,
    node: Node,
}

impl Context {
    pub fn new(node: Node) -> Self {
        Self { node }
    }

    pub fn node(&self) -> Node {
        self.node.clone()
    }
}
