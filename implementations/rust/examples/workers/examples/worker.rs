use ockam::node::Node;
use ockam::worker::Worker;

struct MyWorker {}

struct Data {}

impl Worker<Data> for MyWorker {}

#[ockam::node]
pub async fn main() {
    let node = Node::new();

    let mut worker = ockam::worker::with(node.clone(), MyWorker {});
    let starting = worker.start();

    if let Some(address) = starting.await {
        println!("Started Worker at address {}", address)
    }
}
