use crate::relay_service::relay::Relay;
use crate::{Context, RelayServiceOptions};
use core::str::from_utf8;
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Any, DenyAll, Result, Routed, Worker};
use ockam_node::WorkerBuilder;

/// Alias worker to register remote workers under local names.
///
/// To talk with this worker, you can use the
/// [`RemoteRelay`](crate::remote::RemoteRelay) which is a
/// compatible client for this server.
#[non_exhaustive]
pub struct RelayService {
    options: RelayServiceOptions,
}

impl RelayService {
    /// Start a forwarding service
    pub async fn create(
        ctx: &Context,
        address: impl Into<Address>,
        options: RelayServiceOptions,
    ) -> Result<()> {
        let address = address.into();

        options.setup_flow_control_for_relay_service(ctx.flow_controls(), &address);

        let service_incoming_access_control = options.service_incoming_access_control.clone();

        let s = Self { options };

        WorkerBuilder::new(s)
            .with_address(address)
            .with_incoming_access_control_arc(service_incoming_access_control)
            .with_outgoing_access_control(DenyAll)
            .start(ctx)
            .await?;

        Ok(())
    }
}

#[crate::worker]
impl Worker for RelayService {
    type Context = Context;
    type Message = Any;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let forward_route = msg.return_route();
        let payload = msg.into_transport_message().payload;

        let random_address = Address::random_tagged("Relay.service");

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
            .setup_flow_control_for_relay(ctx.flow_controls(), &address);

        Relay::create(
            ctx,
            address,
            forward_route,
            payload,
            self.options.relays_incoming_access_control.clone(),
        )
        .await?;

        Ok(())
    }
}
