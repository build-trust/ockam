use ockam::{Address, Any, Context, LocalMessage, Result, Routed, Worker};

pub struct Forwarder(pub Address);

#[ockam::worker]
impl Worker for Forwarder {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Some type conversion
        let mut transport_message = msg.into_local_message().into_transport_message();

        transport_message
            .onward_route
            .modify()
            .pop_front() // Remove my address from the onward_route
            .prepend(self.0.clone()); // Prepend predefined address to the onward_route

        // Wipe all local info (e.g. transport types)
        let message = LocalMessage::new(transport_message, vec![]);

        // Send the message on its onward_route
        ctx.forward(message).await
    }
}
