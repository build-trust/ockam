use ockam::{Context, Result, Worker};
use serde::{Deserialize, Serialize};

struct StatefulWorker {
    inner: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Message(usize);

impl Worker for StatefulWorker {
    type Context = Context;
    type Message = Message;

    fn handle_message(&mut self, ctx: &mut Context, msg: Message) -> Result<()> {
        self.inner += msg.0;
        ctx.send_message("app", Message(self.inner)).unwrap();
        Ok(())
    }
}

#[ockam::node]
async fn main(mut context: Context) {
    context
        .start_worker("io.ockam.state", StatefulWorker { inner: 0 })
        .unwrap();

    context
        .send_message("io.ockam.state", Message(1024))
        .unwrap();
    context
        .send_message("io.ockam.state", Message(256))
        .unwrap();
    context.send_message("io.ockam.state", Message(32)).unwrap();

    assert_eq!(context.receive::<Message>().unwrap(), Message(1024));
    assert_eq!(context.receive::<Message>().unwrap(), Message(1280));
    assert_eq!(context.receive::<Message>().unwrap(), Message(1312));

    println!("Received expected worker state");
    context.stop().unwrap();
}
