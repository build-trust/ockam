use ockam::node::Node;
use ockam::worker::{Worker, WorkerContext};
use ockam::OckamResult;

struct PrintWorker {}

#[derive(Debug)]
struct Data {
    val: usize,
}

impl Worker<Data> for PrintWorker {
    fn handle(&self, data: Data, _context: &WorkerContext<Data>) -> OckamResult<bool> {
        println!("{:#?}", data);
        Ok(true)
    }
}

#[ockam::node]
async fn main() {
    let node = Node::new();

    let mut worker = ockam::worker::with(node.clone(), PrintWorker {});
    let starting = worker.address("printer").start();

    if let Some(address) = starting.await {
        if let Ok(n) = node.lock() {
            n.send(&address, Data { val: 123 });
        }
    }
}
