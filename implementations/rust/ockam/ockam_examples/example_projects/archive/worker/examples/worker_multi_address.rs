use ockam::{Context, Result, Routed, Worker};

struct MultiAddressWorker;

#[ockam::worker]
impl Worker for MultiAddressWorker {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Worker main address: '{}'", ctx.address());
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Addr '{}' received: '{}'", ctx.address(), msg);
        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker(
        vec!["addr.main", "addr.input", "addr.output"],
        MultiAddressWorker,
    )
    .await?;

    ctx.send("addr.main", String::from("Hi")).await?;
    ctx.send("addr.input", String::from("Hi")).await?;
    ctx.send("addr.output", String::from("Hi")).await?;

    ctx.stop().await
}
