use ockam::{async_worker, Context, Passthrough, Result, Routed, Worker};

struct MyWorker;

#[async_worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Passthrough;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let transport = msg.to_transport();

        println!("TransportMessage onward: {:?}", transport.onward_route);
        println!("TransportMessage return: {:?}", transport.return_route);
        println!("TransportMessage payload: {:?}", transport.payload);

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
