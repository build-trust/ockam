use ockam::node::Node;
use ockam::worker::{Worker, WorkerContext};

use ockam::address::Addressable;
use ockam::Result;

struct BuiltWorker {}

struct Data {}

impl Worker<Data> for BuiltWorker {
    fn starting(&self, context: &WorkerContext<Data>) -> Result<bool> {
        println!("Started on address {}", context.address());
        Ok(true)
    }
}

#[ockam::node]
pub async fn main() {
    let node_handle = Node::new();

    let address = ockam::worker::with(node_handle, BuiltWorker {})
        .address("worker123")
        .start();

    match address {
        Some(a) => println!("Node running at address {}", a),
        None => panic!("Failed to start"),
    }
}
