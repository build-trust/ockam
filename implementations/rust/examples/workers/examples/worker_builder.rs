use ockam::node::Node;
use ockam::worker::Worker;

struct BuiltWorker {}

struct Data {}

impl Worker<Data> for BuiltWorker {}

#[ockam::node]
pub async fn main() {
    let node_handle = Node::new();

    let mut worker = ockam::worker::with(node_handle, BuiltWorker {});
    let address = worker.address("worker123").start();

    match address.await {
        Some(a) => println!("Node running at address {}", a),
        None => panic!("Failed to start"),
    }
}
