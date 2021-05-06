use ockam::{Context, Result, Routed, Worker};
use serde::{Deserialize, Serialize};
use tracing::info;

struct Echo;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Number(u8);

#[ockam::worker]
impl Worker for Echo {
    type Context = Context;
    type Message = Number;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Number>) -> Result<()> {
        let addr = msg.return_route();
        let msg = msg.body();

        // Send three messages back, but only have the third message
        // be the 'correct' value.
        ctx.send(addr.clone(), Number(msg.0 - 2)).await?;
        ctx.send(addr.clone(), Number(msg.0 - 1)).await?;
        ctx.send(addr.clone(), Number(msg.0)).await?;
        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let num = Number(5);

    // Start the echo service
    ctx.start_worker("echo", Echo).await?;

    // Send a message
    ctx.send("echo", num.clone()).await?;

    // Wait for the 'correct' reply
    let reply = ctx.receive_match::<Number, _>(|msg| msg == &num).await?;
    info!("Received correct reply: {:?}", reply);

    ctx.stop().await
}
