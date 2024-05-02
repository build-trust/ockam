use crate::puncture::negotiation::message::UdpPunctureNegotiationMessage;
use crate::puncture::negotiation::options::UdpPunctureNegotiationListenerOptions;
use crate::puncture::negotiation::worker::UdpPunctureNegotiationWorker;
use crate::puncture::rendezvous_service::RendezvousClient;
use crate::{UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam_core::{async_trait, Address, AllowAll, Any, Decodable, Result, Route, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};
use tracing::info;

/// UDP puncture listener
pub struct UdpPunctureNegotiationListener {
    udp: UdpTransport,
    rendezvous_route: Route,
}

impl UdpPunctureNegotiationListener {
    /// Create and start a new listener on given address
    pub async fn create(
        ctx: &Context,
        address: impl Into<Address>,
        udp: &UdpTransport,
        rendezvous_route: Route,
        options: UdpPunctureNegotiationListenerOptions,
    ) -> Result<()> {
        let address = address.into();

        let access_control = options.incoming_access_control.clone();

        options.setup_flow_control_for_listener(ctx.flow_controls(), &address);

        let worker = Self {
            udp: udp.clone(),
            rendezvous_route,
        };

        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control_arc(access_control)
            // TODO: PUNCTURE replace with DenyAll when we pass message to the spawned worker as
            //  an argument instead of sending
            .with_outgoing_access_control(AllowAll)
            .start(ctx)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for UdpPunctureNegotiationListener {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        info!("Received a UDP puncture request");

        let src_addr = msg.src_addr();
        let msg_payload = UdpPunctureNegotiationMessage::decode(msg.payload())?;

        if let UdpPunctureNegotiationMessage::Initiate { .. } = msg_payload {
            let address = Address::random_tagged("UdpPunctureNegotiator.responder");

            if let Some(producer_flow_control_id) = ctx
                .flow_controls()
                .get_flow_control_with_producer(&src_addr)
                .map(|x| x.flow_control_id().clone())
            {
                // Allow a sender with corresponding flow_control_id send messages to this address
                ctx.flow_controls()
                    .add_consumer(address.clone(), &producer_flow_control_id);
            }

            let udp_bind = self
                .udp
                .bind(
                    UdpBindArguments::new().with_bind_address("0.0.0.0:0")?,
                    UdpBindOptions::new(), // FIXME: PUNCTURE
                )
                .await?;
            let client =
                RendezvousClient::new(ctx, &udp_bind, self.rendezvous_route.clone()).await?;

            let worker = UdpPunctureNegotiationWorker::new_responder(&udp_bind, client);

            let msg = msg
                .into_local_message()
                .pop_front_onward_route()?
                .push_front_onward_route(&address);

            ctx.start_worker(address, worker).await?; // FIXME: PUNCTURE Access Control

            // FIXME: PUNCTURE Pass as an argument instead?
            ctx.forward(msg).await?;
        }

        Ok(())
    }
}
