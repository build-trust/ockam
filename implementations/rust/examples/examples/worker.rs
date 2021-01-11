use ockam::message::Message;
use ockam::node::WorkerContext;
use ockam::worker::Worker;
use ockam::Result;

struct MyWorker {}

impl Worker<Message> for MyWorker {
    fn starting(&mut self, context: &mut WorkerContext) -> Result<bool> {
        println!("Started on address {}", context.address());
        Ok(true)
    }

    fn stopping(&mut self, _context: &mut WorkerContext) -> Result<bool> {
        println!("Stopping!");
        Ok(true)
    }
}

#[ockam::node]
pub async fn main() {
    if let Some(address) = ockam::worker::with(MyWorker {}).start() {
        println!("{:?}", address);
    } else {
        panic!("Couldn't start Worker");
    }
}
