use ockam::{Address, Any, Context, Result, Routed, Worker};

pub struct Relay(pub Address);

#[ockam::worker]
impl Worker for Relay {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Some type conversion
        let mut local_message = msg.into_local_message();

        local_message = local_message.replace_front_onward_route(&self.0)?; // Prepend predefined address to the ownward_route

        let prev_hop = local_message.return_route_ref().next()?.clone();

        if let Some(info) = ctx.flow_controls().find_flow_control_with_producer_address(&self.0) {
            ctx.flow_controls()
                .add_consumer(prev_hop.clone(), info.flow_control_id());
        }

        if let Some(info) = ctx.flow_controls().find_flow_control_with_producer_address(&prev_hop) {
            ctx.flow_controls().add_consumer(self.0.clone(), info.flow_control_id());
        }

        // Send the message on its onward_route
        ctx.send_local_message(local_message).await
    }
}
