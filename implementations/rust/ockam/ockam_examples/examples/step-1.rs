use ockam::{Context, Result, Routed, Worker};

struct EchoService;

#[ockam::worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("echo_service", EchoService).await?;

    ctx.send("echo_service", "Hello Ockam!".to_string()).await?;

    let reply = ctx.receive::<String>().await?;
    println!("Reply: {}", reply);

    ctx.stop().await
}
