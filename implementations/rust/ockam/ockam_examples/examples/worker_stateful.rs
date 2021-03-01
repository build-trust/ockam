use ockam::{async_worker, Context, Result, Worker};
use serde::{Deserialize, Serialize};

struct StatefulWorker {
    inner: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Message(usize);

#[async_worker]
impl Worker for StatefulWorker {
    type Context = Context;
    type Message = Message;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Message) -> Result<()> {
        self.inner += msg.0;
        ctx.send_message("app", Message(self.inner)).await?;
        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("io.ockam.state", StatefulWorker { inner: 0 })
        .await?;

    ctx.send_message("io.ockam.state", Message(1024)).await?;
    ctx.send_message("io.ockam.state", Message(256)).await?;
    ctx.send_message("io.ockam.state", Message(32)).await?;

    assert_eq!(ctx.receive::<Message>().await?, Message(1024));
    assert_eq!(ctx.receive::<Message>().await?, Message(1280));
    assert_eq!(ctx.receive::<Message>().await?, Message(1312));

    println!("Received expected worker state");
    ctx.stop().await
}
