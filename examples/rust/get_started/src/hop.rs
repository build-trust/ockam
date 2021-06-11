use ockam::{Any, Context, Result, Routed, Worker};

pub struct Hop;

#[ockam::worker]
impl Worker for Hop {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        let mut local_msg = msg.into_local_message();
        let transport_msg = local_msg.transport_mut();
        transport_msg.onward_route.step()?;
        transport_msg.return_route.modify().prepend(ctx.address());
        ctx.forward(local_msg).await
    }
}
