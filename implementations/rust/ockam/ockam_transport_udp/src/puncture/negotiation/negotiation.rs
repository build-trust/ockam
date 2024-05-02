use crate::puncture::negotiation::worker::UdpPunctureNegotiationWorker;
use crate::puncture::rendezvous_service::RendezvousClient;
use crate::{UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver, Sender};

/// Allows to negotiate a UDP puncture to the other node by communicating
/// with its [`UdpPunctureListener`] via some side channel (e.g. Relayed connection through the
/// Ockam Orchestrator)
pub struct UdpPunctureNegotiation {}

impl UdpPunctureNegotiation {
    /// Start a UDP puncture negotiation and notify whenever puncture is open.
    /// WARNING: Only use Sender to subscribe (get new Receiver instances)
    pub async fn start_negotiation(
        ctx: &Context,
        onward_route: Route, // Route to the UdpPunctureNegotiationListener
        udp: &UdpTransport,
        rendezvous_route: Route,
    ) -> Result<(Receiver<Route>, Sender<Route>)> {
        let next = onward_route.next()?.clone();

        let udp_bind = udp
            .bind(
                UdpBindArguments::new().with_bind_address("0.0.0.0:0")?,
                UdpBindOptions::new(), // FIXME: PUNCTURE
            )
            .await?;

        let client = RendezvousClient::new(ctx, &udp_bind, rendezvous_route).await?;
        let (notify_reachable_sender, notify_reachable_receiver) = broadcast::channel(1);
        let worker = UdpPunctureNegotiationWorker::new_initiator(
            onward_route,
            &udp_bind,
            client,
            notify_reachable_sender.clone(),
        );

        let address = Address::random_tagged("UdpPunctureNegotiator.initiator");

        if let Some(flow_control_id) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            // To be able to receive the response
            ctx.flow_controls()
                .add_consumer(address.clone(), &flow_control_id);
        }

        ctx.start_worker(address, worker).await?; // FIXME: PUNCTURE Access Control

        Ok((notify_reachable_receiver, notify_reachable_sender))
    }
}
