use crate::Context;
use core::str::from_utf8;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{
    Address, AllowOnwardAddress, Any, DenyAll, IncomingAccessControl, LocalMessage, Result, Route,
    Routed, TransportMessage, Worker,
};
use ockam_node::WorkerBuilder;
use tracing::info;

/// Alias worker to register remote workers under local names.
///
/// To talk with this worker, you can use the
/// [`RemoteForwarder`](crate::remote::RemoteForwarder) which is a
/// compatible client for this server.
#[non_exhaustive]
pub struct ForwardingService {
    forwarders_incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl ForwardingService {
    /// Start a forwarding service
    pub async fn create(
        ctx: &Context,
        address: impl Into<Address>,
        service_incoming_access_control: impl IncomingAccessControl,
        forwarders_incoming_access_control: impl IncomingAccessControl,
    ) -> Result<()> {
        let s = Self {
            forwarders_incoming_access_control: Arc::new(forwarders_incoming_access_control),
        };
        ctx.start_worker(address.into(), s, service_incoming_access_control, DenyAll)
            .await?;
        Ok(())
    }
}

#[crate::worker]
impl Worker for ForwardingService {
    type Context = Context;
    type Message = Any;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let forward_route = msg.return_route();
        let payload = msg.into_transport_message().payload;
        Forwarder::create(
            ctx,
            forward_route,
            payload,
            self.forwarders_incoming_access_control.clone(),
        )
        .await?;

        Ok(())
    }
}

struct Forwarder {
    forward_route: Route,
    // this option will be `None` after this worker is initialized, because
    // while initializing, the worker will send the payload contained in this
    // field to the `forward_route`, to indicate a successful connection
    payload: Option<Vec<u8>>,
}

impl Forwarder {
    async fn create(
        ctx: &Context,
        forward_route: Route,
        registration_payload: Vec<u8>,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        let random_address = Address::random_tagged("Forwarder.service");

        // TODO: assume that the first byte is length, ignore it.
        // We have to improve this actually parse the payload.
        let address = match registration_payload.get(1..) {
            Some(address) => match from_utf8(address) {
                Ok(v) => Address::from_string(v),
                Err(_e) => random_address,
            },
            None => random_address,
        };
        info!("Created new alias for {}", forward_route);

        let next_hop = forward_route.next()?.clone();
        let forwarder = Self {
            forward_route,
            payload: Some(registration_payload.clone()),
        };

        WorkerBuilder::with_access_control(
            incoming_access_control,
            Arc::new(AllowOnwardAddress(next_hop)), // TODO: @ac we can actually check not only the next hop, but that the whole forward_route is the beginning of a onward_route
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

        ctx.forward(message).await
    }
}
