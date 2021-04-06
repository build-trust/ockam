use ockam::{async_worker, Context, Result, Routed, Worker, AnyMessage};

struct MyWorker;

#[async_worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = AnyMessage;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // Transport message is accessed via msg.transport()
        // println!("TransportMessage onward: {:?}", msg.onward);
        // println!("TransportMessage return: {:?}", msg.return_);
        // println!("TransportMessage payload: {:?}", msg.payload);

        println!("Received message");

        // ctx.stop().await

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("worker", MyWorker).await?;

    ctx.send_message("worker", "Hello World!".to_string())
        .await?;
    ctx.send_message("worker", [0u8; 32])
        .await?;
    ctx.send_message("worker", 5)
        .await?;

    Ok(())
}
