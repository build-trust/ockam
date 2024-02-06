use ockam::{Any, Context, Result, Routed, Worker};

// TODO: Split into two workers to avoid cycles + there are many implementations of Hop worker, fix all of them
pub struct Hop;

#[ockam::worker]
impl Worker for Hop {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // Send the message on its onward_route
        ctx.forward(msg.into_local_message()).await
    }
}
