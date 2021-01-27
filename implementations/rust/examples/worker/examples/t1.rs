use ockam::{Worker, Context, WorkerBuilder};

#[derive(Debug)]
struct TestWorker {}

#[derive(Clone)]
struct Data {}

impl Worker<Data> for TestWorker {
    fn starting(&mut self, _context: &Context<Data>) {
        println!("TestWorker starting at {}", _context.address)
    }

    fn stopping(&mut self) {
        println!("TestWorker stopping")
    }

    fn handle(&mut self, _data: Data, _context: &Context<Data>) {
        unimplemented!()
    }
}

// TODO not
#[ockam::node]
async fn main(context: ockam::Context<Data>) {
    let node = context.node;

    let test_worker = TestWorker {};

    let mut builder = WorkerBuilder::new(test_worker);
    let worker = builder.on(node.clone()).at("test").start();
    worker.await;

    node.stop().await.unwrap();
}
