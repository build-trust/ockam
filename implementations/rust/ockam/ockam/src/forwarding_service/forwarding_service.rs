use crate::forwarding_service::forwarder::Forwarder;
use crate::{Context, ForwardingServiceOptions};
use core::str::from_utf8;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Any, DenyAll, Result, Routed, Worker};
use ockam_node::WorkerBuilder;

/// Alias worker to register remote workers under local names.
///
/// To talk with this worker, you can use the
/// [`RemoteForwarder`](crate::remote::RemoteForwarder) which is a
/// compatible client for this server.
#[non_exhaustive]
pub struct ForwardingService {
    options: ForwardingServiceOptions,
}

impl ForwardingService {
    /// Start a forwarding service
    pub async fn create(
        ctx: &Context,
        address: impl Into<Address>,
        options: ForwardingServiceOptions,
    ) -> Result<()> {
        let address = address.into();

        options.setup_flow_control_for_forwarding_service(ctx.flow_controls(), &address);

        let service_incoming_access_control = options.service_incoming_access_control.clone();

        let s = Self { options };

        WorkerBuilder::with_access_control(
            service_incoming_access_control,
            Arc::new(DenyAll),
            address,
            s,
        )
        .start(ctx)
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

        let random_address = Address::random_tagged("Forwarder.service");

        // TODO: assume that the first byte is length, ignore it.
        // We have to improve this actually parse the payload.
        let address = match payload.get(1..) {
            Some(address) => match from_utf8(address) {
                Ok(v) if v != "register" => Address::from_string(v),
                _ => random_address,
            },
            None => random_address,
        };

        self.options
            .setup_flow_control_for_forwarder(ctx.flow_controls(), &address);

        Forwarder::create(
            ctx,
            address,
            forward_route,
            payload,
            self.options.forwarders_incoming_access_control.clone(),
        )
        .await?;

        Ok(())
    }
}
