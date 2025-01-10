use crate::puncture::negotiation::message::{
    UdpPunctureNegotiationMessageAcknowledge, UdpPunctureNegotiationMessageInitiate,
};
use crate::puncture::negotiation::options::UdpPunctureNegotiationListenerOptions;
use crate::puncture::rendezvous_service::RendezvousClient;
use crate::{UdpBindArguments, UdpBindOptions, UdpPuncture, UdpPunctureOptions, UdpTransport};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{async_trait, Address, AllowAll, DenyAll, Result, Route, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};
use tracing::{error, info};

/// UDP puncture listener
pub struct UdpPunctureNegotiationListener {
    udp: UdpTransport,
    rendezvous_route: Route,
    flow_control_id: FlowControlId,
}

impl UdpPunctureNegotiationListener {
    /// Create and start a new listener on given address
    pub fn create(
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
            flow_control_id: options.flow_control_id,
        };

        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control_arc(access_control)
            .with_outgoing_access_control(DenyAll)
            .start(ctx)?;

        Ok(())
    }

    async fn start_puncture(
        ctx: Context,
        udp: UdpTransport,
        rendezvous_route: Route,
        flow_control_id: FlowControlId,
        msg: UdpPunctureNegotiationMessageInitiate,
        return_route: Route,
    ) -> Result<()> {
        // We create a new bind for each puncture. Ownership will be transferred to the
        // UdpPunctureReceiverWorker which is responsible for stopping it eventually
        // TODO: Consider limiting incoming access control for that bind
        let udp_bind = udp
            .bind(
                UdpBindArguments::new().with_bind_address("0.0.0.0:0")?,
                UdpBindOptions::new(),
            )
            .await?;

        let client = RendezvousClient::new(&udp_bind, rendezvous_route);
        let my_udp_public_address = match client.get_my_address(&ctx).await {
            Ok(my_udp_public_address) => my_udp_public_address,
            Err(err) => {
                error!(
                    "Error getting UDP public address for the responder: {}",
                    err
                );
                udp.unbind(udp_bind.sender_address())?;
                return Err(err);
            }
        };

        let initiator_remote_address = Address::from(msg.initiator_remote_address);

        let options = UdpPunctureOptions::new_with_spawner(flow_control_id);

        // Let's start puncture as we received the initiates
        let my_remote_address =
            Address::random_tagged("UdpPunctureNegotiationWorker.remote.responder");
        UdpPuncture::create(
            &ctx,
            udp_bind,
            msg.initiator_udp_public_address,
            my_remote_address.clone(),
            initiator_remote_address,
            options,
            // We can't send messages to the remote address of `UdpPunctureReceiverWorker`
            // on the other side, since it's not started yet, so we'll just send ping
            // messages to the corresponding UDP transport worker of that node, the messages
            // will be just dropped on that side, but the fact that we send them will keep
            // the "connection" open
            // After we receive the first ping, which guarantees
            // that `UdpPunctureReceiverWorker` was started on the other side, we'll start
            // sending messages to that worker
            true,
        )?;

        // Send Acknowledge back, so that initiator will start the puncture as well
        ctx.send(
            return_route,
            UdpPunctureNegotiationMessageAcknowledge {
                responder_udp_public_address: my_udp_public_address,
                responder_remote_address: my_remote_address.to_vec(),
            },
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for UdpPunctureNegotiationListener {
    type Message = UdpPunctureNegotiationMessageInitiate;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        info!("Received a UDP puncture request");

        let return_route = msg.return_route().clone();
        let msg = msg.into_body()?;

        let child_ctx = ctx.new_detached(
            Address::random_tagged("UdpPunctureNegotiator.responder"),
            DenyAll,
            AllowAll,
        )?;

        let rendezvous_route = self.rendezvous_route.clone();
        let udp = self.udp.clone();
        let flow_control_id = self.flow_control_id.clone();
        tokio::spawn(async move {
            Self::start_puncture(
                child_ctx,
                udp,
                rendezvous_route,
                flow_control_id,
                msg,
                return_route,
            )
            .await
        });

        Ok(())
    }
}
