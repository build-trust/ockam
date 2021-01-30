use ockam::{Context, Handler, Message, Worker, WorkerBuilder};

pub struct Ping;
impl Message for Ping {}

pub struct Echoer;
impl Worker for Echoer {}

impl Handler<Ping> for Echoer {
    fn handle(&mut self, _context: &mut Context, _message: Ping) {
        println!("*** ping!! ***")
    }
}

pub struct Print;
impl Message for Print {}

pub struct Printer;
impl Worker for Printer {}

impl Handler<Print> for Printer {
    fn handle(&mut self, _context: &mut Context, _message: Print) {
        println!("*** print!! ***")
    }
}

#[ockam::node]
async fn main(context: Context) {
    let node = context.node;

    WorkerBuilder::new(Echoer {})
        .on(&node)
        .at("echoer")
        .start()
        .await;
    WorkerBuilder::new(Printer {})
        .on(&node)
        .at("printer")
        .start()
        .await;

    node.stop().await.unwrap();
}
