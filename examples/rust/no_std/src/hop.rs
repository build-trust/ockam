#[cfg(all(not(feature = "std"), feature = "cortexm"))]
use ockam::compat::boxed::Box;
use ockam::{Any, Context, Result, Routed, Worker};

pub struct Hop;

#[ockam::worker]
impl Worker for Hop {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        info!("Address: {}, Received: {}", ctx.address(), msg);

        // Send the message on its onward_route
        ctx.forward(msg.into_local_message()).await
    }
}
