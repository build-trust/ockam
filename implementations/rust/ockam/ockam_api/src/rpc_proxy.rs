use ockam::{Any, Context, Result, Routed, Worker};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{route, Address, AllowAll, LocalMessage, TransportMessage};

#[derive(Default, Debug)]
pub struct RpcProxyService {}

impl RpcProxyService {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Split into two workers to avoid cycles
#[ockam::worker]
impl Worker for RpcProxyService {
    type Context = Context;
    type Message = Any;

    /// This handle function takes any incoming message and forwards
    /// it to the next hop in it's onward route
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // Some type conversion
        let msg = msg.into_local_message();
        let local_info = msg.local_info().to_vec();
        let msg = msg.into_transport_message();

        // Create a dedicated type for this service that would take
        // 1. onward_route
        // 2. payload
        // 3. message send/receive options

        let mut onward_route = msg.onward_route;
        // Remove my address from the onward_route
        onward_route.step()?;
        let next = onward_route.next()?.clone();

        let return_route = msg.return_route;

        let child_address = Address::random_tagged("RpcProxy_child_address");

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_route, route![child_address.clone()], msg.payload),
            local_info,
        );

        let mut child_ctx = ctx
            .new_detached(child_address.clone(), AllowAll, AllowAll)
            .await?;

        if let Some(flow_control_id) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            ctx.flow_controls().add_consumer(
                child_address,
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        // Send the message on its onward_route
        ctx.forward(msg).await?;

        let response = child_ctx.receive::<Any>().await?;
        let response = response.into_local_message();
        let local_info = response.local_info().to_vec();

        let msg = LocalMessage::new(
            TransportMessage::v1(
                return_route,
                route![],
                response.into_transport_message().payload,
            ),
            local_info,
        );

        ctx.forward(msg).await?;

        Ok(())
    }
}
