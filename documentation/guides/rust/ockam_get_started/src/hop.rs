use ockam::{Any, Context, Result, Routed, Worker};

pub struct Hop;

#[ockam::worker]
impl Worker for Hop {
    type Message = Any;
    type Context = Context;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in its onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        let mut msg = msg.into_transport_message();
        msg.onward_route.step()?;
        msg.return_route.modify().prepend(ctx.address());
        ctx.forward(msg).await
    }
}
