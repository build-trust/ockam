use ockam::{Any, NodeContext, Result, Routed, Worker};

pub struct Hop;

#[ockam::worker]
impl<C: NodeContext> Worker<C> for Hop {
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Some type conversion
        let mut message = msg.into_local_message();
        let transport_message = message.transport_mut();

        // Remove my address from the onward_route
        transport_message.onward_route.step()?;

        // Insert my address at the beginning return_route
        transport_message.return_route.modify().prepend(ctx.address());

        // Send the message on its onward_route
        ctx.forward(message).await
    }
}
