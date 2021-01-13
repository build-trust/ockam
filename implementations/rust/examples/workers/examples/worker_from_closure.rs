#[ockam::node]
pub async fn main() {
    if let Some(address) = ockam::worker::with_closure(move |message, context| {
        println!("Address: {}\tMessage: {:#?}", context.address(), message)
    })
    .start()
    {
        ockam::node::send(&address, "hello".into());
        ockam::node::worker_at(&address, |maybe_worker| {
            if let Some(worker) = maybe_worker {
                println!("Worker at {}", worker.address())
            }
        })
    }
}
