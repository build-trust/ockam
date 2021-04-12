use ockam::{async_worker, Any, Context, Result, Route, Routed, Worker};

struct MyRouter;

#[async_worker]
impl Worker for MyRouter {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in the route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let mut msg = msg.into_transport_message();
        msg.onward_route.step().unwrap();
        ctx.forward(msg).await
    }
}

struct MyClient;

#[async_worker]
impl Worker for MyClient {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Received: '{}'", msg);
        ctx.send("app", "ok".to_string()).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("router", MyRouter).await?;
    ctx.start_worker("client", MyClient).await?;

    ctx.send(
        Route::new().append("router").append("client"),
        "Hello Client".to_string(),
    )
    .await?;

    // Wait for a reply from the client worker
    let _ = ctx.receive::<String>().await?;
    ctx.stop().await
}
