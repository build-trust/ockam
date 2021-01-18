use ockam::node::Node;
use ockam::worker::{Worker, WorkerContext};
use ockam::Result;

struct PrintWorker {}

#[derive(Debug)]
struct Data {
    val: usize,
}

impl Worker<Data> for PrintWorker {
    fn handle(&self, data: Data, _context: &WorkerContext<Data>) -> Result<bool> {
        println!("{:#?}", data);
        Ok(true)
    }
}

#[ockam::node]
async fn main() {
    let node = Node::new();
    if let Some(address) = ockam::worker::with(node.clone(), PrintWorker {})
        .address("printer")
        .start()
    {
        println!("Address: {}", address);

        node.borrow().send(&address, Data { val: 123 })
    }
}
