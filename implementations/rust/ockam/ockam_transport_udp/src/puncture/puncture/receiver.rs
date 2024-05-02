use crate::puncture::puncture::message::PunctureMessage;
use crate::puncture::puncture::sender::UdpPunctureSenderWorker;
use crate::puncture::puncture::{Addresses, UdpPunctureOptions};
use crate::{PunctureError, UdpBind, UDP};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    route, Address, AllowAll, AllowSourceAddress, Any, Decodable, DenyAll, LocalMessage, Mailbox,
    Mailboxes, Result, Route, Routed, Worker,
};
use ockam_node::{Context, DelayedEvent, WorkerBuilder};
use std::time::{Duration, Instant};
use tokio::sync::broadcast::Sender;
use tracing::log::warn;
use tracing::{debug, info, trace};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const PUNCTURE_OPEN_TIMEOUT: Duration = Duration::from_secs(10);

// TODO: PUNCTURE Possible future improvement, explicitly send list of possible
//  reachable addresses (usually local IPs) to Rendezvous service, to allow
//  opening punctures to local nodes

/// [`Worker`] for UDP Puncture
///
/// Constantly sends messages to the other side to keep the "connection" in the
/// routing tables (heartbeat). Also, responsible for sending payload from the remote
/// to addresses inside our node.
pub(crate) struct UdpPunctureReceiverWorker {
    /// All Addresses used in this puncture
    addresses: Addresses,
    /// For generating internal heartbeat messages
    heartbeat: DelayedEvent<()>,
    /// Is puncture open?
    puncture_open: bool,
    /// Notify that puncture is open those who wait for it
    notify_puncture_open_sender: Sender<Route>,
    /// Route to peer node's [`UdpPunctureReceiverWorker`]
    peer_route: Route,
    /// Timestamp of most recent message received from peer
    peer_received_at: Instant,
    /// If we have received the first ping
    first_ping_received: bool,
    /// The other node UdpPunctureWorker address
    recipient_address: Address,
    // Will send messages to the UDP transport worker instead of the `UdpPunctureReceiverWorker`
    // on the other side, until we receive the first ping, which guarantees
    // that `UdpPunctureReceiverWorker` was started on the other side
    // See comments at the point of usage
    redirect_first_message_to_transport: bool,
}

impl UdpPunctureReceiverWorker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create(
        ctx: &Context,
        bind: &UdpBind,
        peer_udp_address: String,
        recipient_address: Address,
        addresses: Addresses,
        notify_puncture_open_sender: Sender<Route>,
        options: UdpPunctureOptions,
        redirect_first_message_to_transport: bool,
    ) -> Result<()> {
        let peer_udp_address = Address::new(UDP, peer_udp_address);

        let peer_route = route![bind.sender_address().clone(), peer_udp_address];

        let heartbeat =
            DelayedEvent::create(ctx, addresses.heartbeat_address().clone(), ()).await?;

        let remote_mailbox = Mailbox::new(
            addresses.remote_address().clone(),
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        );

        options.setup_flow_control(ctx.flow_controls(), &addresses, bind.sender_address())?;

        let receiver_mailbox = Mailbox::new(
            addresses.receiver_address().clone(),
            Arc::new(DenyAll),
            options.create_receiver_outgoing_access_control(ctx.flow_controls()),
        );

        let heartbeat_mailbox = Mailbox::new(
            addresses.heartbeat_address().clone(),
            Arc::new(AllowSourceAddress(heartbeat.address())),
            Arc::new(DenyAll),
        );

        let sender_worker = UdpPunctureSenderWorker::new(notify_puncture_open_sender.subscribe());

        WorkerBuilder::new(sender_worker)
            .with_address(addresses.sender_address().clone())
            .with_incoming_access_control(AllowAll)
            .with_outgoing_access_control(AllowAll)
            .start(ctx)
            .await?;

        // Create and start worker
        let worker = Self {
            addresses: addresses.clone(),
            heartbeat,
            puncture_open: false,
            notify_puncture_open_sender,
            peer_route,
            peer_received_at: Instant::now(),
            first_ping_received: false,
            recipient_address,
            redirect_first_message_to_transport,
        };

        WorkerBuilder::new(worker)
            .with_mailboxes(Mailboxes::new(
                remote_mailbox,
                vec![receiver_mailbox, heartbeat_mailbox],
            ))
            .start(ctx)
            .await?;

        Ok(())
    }

    /// Update state to show the puncture to peer is now open
    async fn set_puncture_open(&mut self) -> Result<()> {
        if !self.puncture_open {
            self.puncture_open = true;

            info!("Puncture succeeded. Route={}", self.peer_route);
        }

        // Even if puncture was already open - let's notify everyone that it's still open
        let _ = self.notify_puncture_open_sender.send(route![
            self.peer_route.clone(),
            self.recipient_address.clone()
        ]);

        Ok(())
    }

    /// Handle messages from peer
    async fn handle_peer(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Any>,
        return_route: &Route,
    ) -> Result<()> {
        let msg = PunctureMessage::decode(msg.payload())?;
        trace!("Puncture remote message: {:?}", msg);

        // Record contact with peer, but only for pong and payload message.
        // Ping message doesn't guarantee that the other side is reachable
        let now = Instant::now();

        // Handle message
        match msg {
            PunctureMessage::Ping => {
                self.first_ping_received = true;
                debug!("Received Ping from peer. Will Pong.");
                ctx.send_from_address(
                    return_route.clone(),
                    PunctureMessage::Pong,
                    self.addresses.remote_address().clone(),
                )
                .await?;
            }
            PunctureMessage::Pong => {
                debug!("Received Pong from peer. Setting as puncture is open");
                self.peer_received_at = now;
                self.set_puncture_open().await?;
            }
            PunctureMessage::Payload {
                onward_route,
                mut return_route,
                payload,
            } => {
                trace!("Received Payload from peer. Will forward to local entity");

                let return_route = return_route
                    .modify()
                    .prepend(self.addresses.sender_address().clone())
                    .into();

                // Update routing & payload
                let local_message = LocalMessage::new()
                    .with_onward_route(onward_route)
                    .with_return_route(return_route)
                    .with_payload(payload);

                // Forward
                ctx.forward_from_address(local_message, self.addresses.receiver_address().clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle heartbeat messages
    async fn handle_heartbeat_impl(&mut self, ctx: &mut Context) -> Result<()> {
        debug!(
            "Puncture Heartbeat: puncture_open = {:?}, peer_route = {:?}",
            self.puncture_open, self.peer_route
        );

        // If we have not heard from peer for a while, consider puncture as closed
        if self.puncture_open && self.peer_received_at.elapsed() >= PUNCTURE_OPEN_TIMEOUT {
            warn!("Haven't received pongs from the peer for more than {:?}. Shutting down the puncture.", PUNCTURE_OPEN_TIMEOUT);

            _ = self.notify_puncture_open_sender.send(route![]);

            // Shut down itself
            ctx.stop_worker(self.addresses.remote_address().clone())
                .await?;

            return Ok(());
        }

        // Do keepalive pings to try and keep the puncture open
        trace!("Pinging peer for keepalive");

        // Will send messages to the UDP transport worker instead of the `UdpPunctureReceiverWorker`
        // on the other side, until we receive the first ping, which guarantees
        // that `UdpPunctureReceiverWorker` was started on the other side
        let route = if !self.first_ping_received && self.redirect_first_message_to_transport {
            self.peer_route.clone()
        } else {
            route![self.peer_route.clone(), self.recipient_address.clone()]
        };

        ctx.send_from_address(
            route,
            PunctureMessage::Ping,
            self.addresses.remote_address().clone(),
        )
        .await?;

        Ok(())
    }

    /// Handle heartbeat messages
    async fn handle_heartbeat(&mut self, ctx: &mut Context) -> Result<()> {
        let res = self.handle_heartbeat_impl(ctx).await;

        // Schedule next heartbeat here in case something errors
        self.heartbeat.schedule(HEARTBEAT_INTERVAL).await?;

        res
    }
}

#[ockam_core::worker]
impl Worker for UdpPunctureReceiverWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.heartbeat.schedule(Duration::ZERO).await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.heartbeat.cancel();

        _ = ctx
            .stop_worker(self.addresses.sender_address().clone())
            .await;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let addr = msg.msg_addr();
        if &addr == self.addresses.remote_address() {
            let return_route = msg.return_route();

            self.handle_peer(ctx, msg, &return_route).await?;
        } else if &addr == self.addresses.heartbeat_address() {
            self.handle_heartbeat(ctx).await?;
        } else {
            return Err(PunctureError::Internal)?;
        };

        Ok(())
    }
}
