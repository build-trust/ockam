use ockam::{async_worker, Context, Result, Worker};

struct MultiAddressWorker;

#[async_worker]
impl Worker for MultiAddressWorker {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Worker main address: '{}'", ctx.address());
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: String) -> Result<()> {
        println!("Addr '{}' received: '{}'", ctx.address(), msg);
        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker(
        vec!["addr.main", "addr.input", "addr.output"],
        MultiAddressWorker,
    )
    .await?;

    ctx.send_message("addr.main", String::from("Hi")).await?;
    ctx.send_message("addr.input", String::from("Hi")).await?;
    ctx.send_message("addr.output", String::from("Hi")).await?;

    ctx.stop().await
}
