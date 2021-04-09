use ockam::{async_worker, Any, Context, Result, Routed, Worker};

struct MyWorker;

#[async_worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("TransportMessage onward: {:?}", msg.onward());
        println!("TransportMessage return: {:?}", msg.reply());
        println!("TransportMessage payload: {:?}", msg.payload());

        ctx.stop().await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("worker.middleware", MyWorker).await?;

    ctx.send_message("worker.middleware", "Hello World!".to_string())
        .await?;

    Ok(())
}
