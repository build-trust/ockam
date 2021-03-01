use ockam::{async_worker, Context, Result, Worker};

struct Echoer;

#[async_worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: String) -> Result<()> {
        ctx.send_message("app", format!("{}", msg)).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer).await?;

    ctx.send_message("echoer", "Hello Ockam!".to_string()).await?;

    let reply = ctx.receive::<String>().await?;
    println!("Reply: {}", reply);

    ctx.stop().await
}

