use ockam::message::Message;
use ockam::node::WorkerContext;
use ockam::worker::Worker;
use ockam::Result;

struct PrintWorker {}

impl Worker<Message> for PrintWorker {
    fn handle(&self, message: Message, _context: &mut WorkerContext) -> Result<bool> {
        println!("{:#?}", message);
        Ok(true)
    }
}

#[ockam::node]
async fn main() {
    if let Some(address) = ockam::worker::with(PrintWorker {})
        .address("printer")
        .start()
    {
        println!("Address: {}", address);

        ockam::node::send(&address, "hello".into());
    }
}
