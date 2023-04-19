use crate::Context;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{
    Address, AllowAll, AllowOnwardAddress, Any, IncomingAccessControl, LocalMessage,
    OutgoingAccessControl, Result, Route, Routed, TransportMessage, Worker,
};
use ockam_node::WorkerBuilder;
use tracing::info;

pub(super) struct Forwarder {
    forward_route: Route,
    // this option will be `None` after this worker is initialized, because
    // while initializing, the worker will send the payload contained in this
    // field to the `forward_route`, to indicate a successful connection
    payload: Option<Vec<u8>>,
}

impl Forwarder {
    pub(super) async fn create(
        ctx: &Context,
        address: Address,
        forward_route: Route,
        registration_payload: Vec<u8>,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        info!("Created new alias {} for {}", address, forward_route);

        // Should be able to reach last and second last hops
        let outgoing_access_control: Arc<dyn OutgoingAccessControl> = if forward_route.len() == 1 {
            // We are accessed with our node, no transport is involved
            Arc::new(AllowAll)
        } else {
            let next_hop = forward_route.next()?.clone();
            Arc::new(AllowOnwardAddress(next_hop))
        };

        let forwarder = Self {
            forward_route,
            payload: Some(registration_payload.clone()),
        };

        WorkerBuilder::with_access_control(
            incoming_access_control,
            outgoing_access_control,
            address,
            forwarder,
        )
        .start(ctx)
        .await?;

        Ok(())
    }
}

#[crate::worker]
impl Worker for Forwarder {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let payload = self
            .payload
            .take()
            .expect("payload must be available on init");
        let msg = TransportMessage::v1(self.forward_route.clone(), ctx.address(), payload);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        // Remove the last hop so that just route to the node itself is left
        self.forward_route.modify().pop_back();

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let mut message = msg.into_local_message();
        let transport_message = message.transport_mut();

        // Remove my address from the onward_route
        transport_message.onward_route.step()?;

        // Prepend forward route
        transport_message
            .onward_route
            .modify()
            .prepend_route(self.forward_route.clone());

        let next_hop = transport_message.onward_route.next()?.clone();
        let prev_hop = transport_message.return_route.next()?.clone();

        if let Some(info) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next_hop)
        {
            ctx.flow_controls().add_consumer(
                prev_hop.clone(),
                info.flow_control_id(),
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        if let Some(info) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&prev_hop)
        {
            ctx.flow_controls().add_consumer(
                next_hop,
                info.flow_control_id(),
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        ctx.forward(message).await
    }
}
