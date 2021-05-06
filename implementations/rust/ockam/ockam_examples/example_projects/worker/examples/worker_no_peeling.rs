use ockam::{Any, Context, Result, Routed, Worker};

struct MyWorker;

#[ockam::worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("TransportMessage onward: {:?}", msg.onward_route());
        println!("TransportMessage return: {:?}", msg.return_route());
        println!("TransportMessage payload: {:?}", msg.payload());

        ctx.stop().await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("worker.middleware", MyWorker).await?;

    ctx.send("worker.middleware", "Hello World!".to_string())
        .await?;

    Ok(())
}
