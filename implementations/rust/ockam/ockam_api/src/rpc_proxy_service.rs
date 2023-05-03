use ockam::{Any, Context, Result, Routed, Worker};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{route, Address, AllowAll, LocalMessage, TransportMessage};

/// This service allows `ockam_command` to send messages on behalf of a background node
/// This background should run RpcProxyService and expose it through the localhost listener
#[derive(Default, Debug)]
pub struct RpcProxyService {}

impl RpcProxyService {
    pub fn new() -> Self {
        Self {}
    }

    async fn send_and_receive_task(child_ctx: Context, msg: Routed<Any>) {
        let address = child_ctx.address();
        let res = Self::send_and_receive(child_ctx, msg).await;

        if let Some(err) = res.err() {
            warn!(
                "Error occurred while using RpcProxyService at {} err: {}",
                address, err
            );
        }
    }

    async fn send_and_receive(mut child_ctx: Context, msg: Routed<Any>) -> Result<()> {
        // Some type conversion
        let msg = msg.into_local_message();
        let local_info = msg.local_info().to_vec();
        let msg = msg.into_transport_message();

        let mut onward_route = msg.onward_route;
        // Remove my address from the onward_route
        onward_route.step()?;
        let next = onward_route.next()?.clone();

        let return_route = msg.return_route;

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_route,                // Forward message to the intended receiver
                route![child_ctx.address()], // We want the response back to us
                msg.payload,
            ),
            local_info,
        );

        // Add ourself as a Consumer if needed, to be able to receive the response
        if let Some(flow_control_id) = child_ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            child_ctx.flow_controls().add_consumer(
                child_ctx.address(),
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        // Send the request to the intended destination
        child_ctx.forward(msg).await?;

        // Wait for the response
        let response = child_ctx.receive::<Any>().await?;
        let response = response.into_local_message();
        let local_info = response.local_info().to_vec();

        let msg = LocalMessage::new(
            TransportMessage::v1(
                return_route, // Send the response to the original requester
                route![],     // We don't want another request
                response.into_transport_message().payload,
            ),
            local_info,
        );

        // Forward the response to the original requester
        child_ctx.forward(msg).await?;

        Ok(())
    }
}

#[ockam::worker]
impl Worker for RpcProxyService {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // Create a random detached Context to forward the received message, wait for the response
        // and forward it back
        let child_address = Address::random_tagged("RpcProxy_responder");

        let child_ctx = ctx
            .new_detached(child_address.clone(), AllowAll, AllowAll)
            .await?;

        // Spawn a dedicated tak to not block this server
        ctx.runtime()
            .spawn(Self::send_and_receive_task(child_ctx, msg));

        Ok(())
    }
}
