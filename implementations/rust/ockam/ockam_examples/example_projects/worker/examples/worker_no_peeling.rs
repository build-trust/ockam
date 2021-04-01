use ockam::{async_worker, Context, Result, Routed, TransportMessage, Worker};

struct MyWorker;

#[async_worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = TransportMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.use_peeling(false);
        println!("Disable message peeling...");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        println!("TransportMessage onward: {:?}", msg.onward);
        println!("TransportMessage return: {:?}", msg.return_);
        println!("TransportMessage payload: {:?}", msg.payload);

        ctx.stop().await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("worker", MyWorker).await?;

    ctx.send_message("worker", "Hello World!".to_string())
        .await?;

    Ok(())
}
