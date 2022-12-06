use crate::Context;
use core::str::from_utf8;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{
    AccessControl, Address, AllowAll, Any, LocalMessage, Result, Route, Routed, TransportMessage,
    Worker,
};
use tracing::info;

/// Alias worker to register remote workers under local names.
///
/// To talk with this worker, you can use the
/// [`RemoteForwarder`](crate::remote::RemoteForwarder) which is a
/// compatible client for this server.
#[non_exhaustive]
pub struct ForwardingService;

impl ForwardingService {
    /// Start a forwarding service. The address of the forwarding service will be
    /// `"forwarding_service"`.
    pub async fn create(
        ctx: &Context,
        incoming_access_control: Arc<dyn AccessControl>,
        outgoing_access_control: Arc<dyn AccessControl>,
    ) -> Result<()> {
        ctx.start_worker(
            "forwarding_service",
            Self,
            incoming_access_control,
            outgoing_access_control,
        )
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
        Forwarder::create(ctx, forward_route, payload).await?;

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

        let forwarder = Self {
            forward_route,
            payload: Some(registration_payload.clone()),
        };

        ctx.start_worker(
            address,
            forwarder,
            Arc::new(AllowAll), // FIXME: @ac
            Arc::new(AllowAll), // FIXME: @ac
        )
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
