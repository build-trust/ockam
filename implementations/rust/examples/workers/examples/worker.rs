use ockam::address::Addressable;
use ockam::node::Node;
use ockam::worker::{Worker, WorkerContext};
use ockam::Result;

struct MyWorker {}

struct Data {}

impl Worker<Data> for MyWorker {
    fn starting(&self, context: &WorkerContext<Data>) -> Result<bool> {
        println!("Started on address {}", context.address());
        Ok(true)
    }

    fn stopping(&self, _context: &WorkerContext<Data>) -> Result<bool> {
        println!("Stopping!");
        Ok(true)
    }
}

#[ockam::node]
pub async fn main() {
    let node = Node::new();

    if let Some(address) = ockam::worker::with(node.clone(), MyWorker {}).start() {
        println!("Worker started at {:?}", address);

        let n = node.borrow();
        n.stop(&address);
    } else {
        panic!("Couldn't start Worker");
    }
}
