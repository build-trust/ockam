use ockam_node::Node;

fn main() {
    // create node
    let mut node = Node::new().unwrap();
    node.run();
}
