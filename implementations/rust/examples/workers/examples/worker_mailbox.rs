use ockam::address::Address;
use ockam::message::Message;
use ockam::node::WorkerContext;

#[ockam::node]
pub async fn main() {
    let message_queue = ockam::message::new_message_queue(Address::from("worker_inbox"));

    let handler = |message: &Message, context: &mut WorkerContext| {
        println!("Address: {}, Message: {:#?}", context.address, message);
    };

    if let Some(address) = ockam::worker::with_closure(handler)
        .mailbox(message_queue)
        .address("external")
        .start()
    {
        ockam::node::send(&address, "hello".into())
    }
}
