use crate::puncture::negotiation::message::{
    UdpPunctureNegotiationMessageAcknowledge, UdpPunctureNegotiationMessageInitiate,
};
use crate::puncture::rendezvous_service::RendezvousClient;
use crate::{UdpBindArguments, UdpBindOptions, UdpPuncture, UdpPunctureOptions, UdpTransport};
use ockam_core::{Address, AllowAll, Result, Route};
use ockam_node::{Context, MessageReceiveOptions};
use std::time::Duration;
use tracing::{debug, error, info};

/// Allows to negotiate a UDP puncture to the other node by communicating
/// with its [`UdpPunctureListener`] via some side channel (e.g. Relayed connection through the
/// Ockam Orchestrator)
pub struct UdpPunctureNegotiation {}

impl UdpPunctureNegotiation {
    /// Start a UDP puncture negotiation and notify whenever puncture is open.
    pub async fn start_negotiation(
        ctx: &Context,
        onward_route: Route, // Route to the UdpPunctureNegotiationListener
        udp: &UdpTransport,
        rendezvous_route: Route,
        acknowledgment_timeout: Duration,
    ) -> Result<UdpPuncture> {
        let next = onward_route.next()?.clone();

        let address = Address::random_tagged("UdpPunctureNegotiator.initiator");
        let mut child_ctx = ctx.new_detached(address, AllowAll, AllowAll)?;

        if let Some(flow_control_id) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            // To be able to receive the response
            ctx.flow_controls()
                .add_consumer(child_ctx.primary_address(), &flow_control_id);
        }

        // We create a new bind for each puncture. Ownership will be transferred to the
        // UdpPunctureReceiverWorker which is responsible for stopping it eventually
        // TODO: Consider limiting incoming access control for that bind
        let udp_bind = udp
            .bind(
                UdpBindArguments::new().with_bind_address("0.0.0.0:0")?,
                UdpBindOptions::new(),
            )
            .await?;

        debug!(
            "Initializing UdpPunctureNegotiation Initiator at {}",
            child_ctx.primary_address()
        );
        let client = RendezvousClient::new(&udp_bind, rendezvous_route);
        let my_udp_public_address = match client.get_my_address(ctx).await {
            Ok(my_udp_public_address) => my_udp_public_address,
            Err(err) => {
                error!(
                    "Error getting UDP public address for the initiator: {}",
                    err
                );
                udp.unbind(udp_bind.sender_address())?;
                return Err(err);
            }
        };

        info!(
            "UdpPunctureNegotiation Initiator {} got its public address: {}",
            child_ctx.primary_address(),
            my_udp_public_address
        );

        // Send Initiate message to the responder, but don't start actual UDP puncture yet,
        // until we receive Acknowledge from them
        let my_remote_address =
            Address::random_tagged("UdpPunctureNegotiationWorker.remote.initiator");
        child_ctx
            .send(
                onward_route,
                UdpPunctureNegotiationMessageInitiate {
                    initiator_udp_public_address: my_udp_public_address,
                    initiator_remote_address: my_remote_address.to_vec(),
                },
            )
            .await?;

        let response = match child_ctx
            .receive_extended::<UdpPunctureNegotiationMessageAcknowledge>(
                MessageReceiveOptions::new().with_timeout(acknowledgment_timeout),
            )
            .await
        {
            Ok(response) => response,
            Err(err) => {
                error!(
                    "Error receiving response for Udp Puncture at: {}. {}",
                    child_ctx.primary_address(),
                    err
                );

                udp.unbind(udp_bind.sender_address())?;
                return Err(err);
            }
        };

        let response = match response.into_body() {
            Ok(response) => response,
            Err(err) => {
                error!(
                    "Invalid response for Udp Puncture at: {}. {}",
                    child_ctx.primary_address(),
                    err
                );

                udp.unbind(udp_bind.sender_address())?;
                return Err(err);
            }
        };

        let options = UdpPunctureOptions::new();

        // Start puncture
        let puncture = UdpPuncture::create(
            ctx,
            udp_bind,
            response.responder_udp_public_address,
            my_remote_address.clone(),
            Address::from(response.responder_remote_address),
            options,
            false,
        )?;

        Ok(puncture)
    }
}
