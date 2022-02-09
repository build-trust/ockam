use ockam::{Any, Context, LocalMessage, Result, Route, Routed, Worker};

struct MyRouter;

#[ockam::worker]
impl Worker for MyRouter {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in the route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Received: {}", msg);
        let mut msg = msg.into_transport_message();
        msg.onward_route.step()?;
        msg.return_route.modify().prepend(ctx.address());
        ctx.forward(LocalMessage::new(msg, vec![])).await
    }
}

struct Echo;

#[ockam::worker]
impl Worker for Echo {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Received: '{}'", msg);
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("router", MyRouter).await?;
    ctx.start_worker("client", Echo).await?;

    ctx.send(
        Route::new().append("router").append("client"),
        "This is an echo".to_string(),
    )
    .await?;

    // Wait for a reply from the client worker
    let msg = ctx.receive::<String>().await?;
    println!("Received message: {}", msg);

    ctx.stop().await
}
