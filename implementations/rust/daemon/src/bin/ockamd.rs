use ockamd::cli::Args;
use ockamd::node::Node;

fn main() {
    let (node, _) = Node::new(Args::parse().into());
    node.run();
}
