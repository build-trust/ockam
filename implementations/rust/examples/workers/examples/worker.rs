use ockam::node::Node;
use ockam::worker::{Handler, Starting, Stopping, Worker};

struct MyWorker {}

struct Data {}

impl Starting<Data> for MyWorker {}
impl Stopping<Data> for MyWorker {}
impl Handler<Data> for MyWorker {}

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
