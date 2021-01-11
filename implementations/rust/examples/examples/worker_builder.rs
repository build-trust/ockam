use ockam::message::Message;
use ockam::node::WorkerContext;
use ockam::worker::Worker;

use ockam::Result;

struct BuiltWorker {}

impl Worker<Message> for BuiltWorker {
    fn starting(&mut self, context: &mut WorkerContext) -> Result<bool> {
        println!("Started on address {}", context.address);
        Ok(true)
    }
}

#[ockam::node]
pub async fn main() {
    let address = ockam::worker::with(BuiltWorker {})
        .address("worker123")
        .start();

    match address {
        Some(a) => println!("Node running at address {}", a),
        None => panic!("Failed to start"),
    }
}
