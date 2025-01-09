use ockam::{Any, Context, Result, Route, Routed, Worker};

pub struct Relay {
    route: Route,
}

impl Relay {
    pub fn new(route: impl Into<Route>) -> Self {
        let route = route.into();

        if route.is_empty() {
            panic!("Relay can't forward messages to an empty route");
        }

        Self { route }
    }
}

#[ockam::worker]
impl Worker for Relay {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        println!("Address: {}, Received: {:?}", ctx.primary_address(), msg);

        let next_on_route = self.route.next()?.clone();

        // Some type conversion
        let mut local_message = msg.into_local_message();

        local_message = local_message.pop_front_onward_route()?;
        local_message = local_message.prepend_front_onward_route(self.route.clone()); // Prepend predefined route to the onward_route

        let prev_hop = local_message.return_route().next()?.clone();

        if let Some(info) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next_on_route)
        {
            ctx.flow_controls().add_consumer(&prev_hop, info.flow_control_id());
        }

        if let Some(info) = ctx.flow_controls().find_flow_control_with_producer_address(&prev_hop) {
            ctx.flow_controls().add_consumer(&next_on_route, info.flow_control_id());
        }

        // Send the message on its onward_route
        ctx.forward(local_message).await
    }
}
