use ockam::{Any, Context, Result, Routed, Worker};

pub struct Hop;

#[ockam::worker]
impl Worker for Hop {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {:?}", ctx.primary_address(), msg);

        // Send the message to the next worker on its onward_route
        ctx.forward(msg.into_local_message().step_forward(ctx.primary_address().clone())?)
            .await
    }
}
