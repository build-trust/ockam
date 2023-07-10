use crate::hole_puncher::message::PunchMessage;
use crate::rendezvous_service::{RendezvousRequest, RendezvousResponse};
use crate::PunchError;
use ockam_core::{
    Address, AllowAll, Any, Decodable, Encodable, LocalMessage, Mailbox, Mailboxes, Result, Route,
    Routed, Worker,
};
use ockam_node::{Context, DelayedEvent, MessageSendReceiveOptions, WorkerBuilder};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, trace};

const HEARTBEAT_EVERY: Duration = Duration::from_secs(1);
const HOLE_OPEN_TIMEOUT: Duration = Duration::from_secs(20);
const PING_TRIES: usize = 5;

// UDP and NAT Hole Punching are unreliable protocols. Expect send and receive
// failures and don't wait too long for them
const QUICK_TIMEOUT: Duration = Duration::from_secs(3);

// TODO: Maybe, implement buffering of messages when hole is not open?

// TODO: Possible future improvement, explicitly send list of possible
// reachable addresses (usually local IPs) to Rendezvous service, to allow
// opening holes to local nodes

// TODO: Should `hole_puncher` and `rendezvous_service` files be moved to their
// own crate outside  `ockam_transport_udp`?

/// [`Worker`] for UDP NAT Hole Puncher
///
/// Using a remote Rendezvous service [`UdpRendezvousService`](crate::rendezvous_service::UdpRendezvousService`) tries to create
/// one half of a bi-directional NAT hole with a remote Hole Puncher.
///
/// See documentation for [`UdpHolePuncher`](crate::hole_puncher::UdpHolePuncher).
///
/// # 'Main' Mailbox
///
/// Sends to...
/// - the remote peer's puncher [`UdpHolePunchWorker`]
/// - the remote Rendezvous service
/// - this puncher's local handle [`UdpHolePuncher`](crate::hole_puncher::UdpHolePuncher)
///
/// Receives from...
/// - the remote peer's puncher [`UdpHolePunchWorker`]
/// - this puncher's local handle [`UdpHolePuncher`](crate::hole_puncher::UdpHolePuncher)
///
/// # 'Local' Mailbox
///
/// Sends and receives to and from entities in the local node.
///
/// Messages received by the 'local' mailbox are sent on to the peer's
/// puncher [`UdpHolePunchWorker`] from the 'main' mailbox.
///
/// Messages received from the peer's puncher [`UdpHolePunchWorker`]
/// by the 'main' mailbox are forwarded to local entities from
/// the 'local' mailbox.
pub(crate) struct UdpHolePunchWorker {
    /// Address of main mailbox
    main_addr: Address,
    /// Address of local mailbox
    local_addr: Address,
    /// Address of our handle's mailbox
    handle_addr: Address,
    /// For generating internal heartbeat messages
    heartbeat: DelayedEvent<PunchMessage>,
    /// Route to Rendezvous service
    rendezvous_route: Route,
    /// Name of this puncher
    this_puncher_name: String,
    /// Name of peer node's puncher
    peer_puncher_name: String,
    /// Is hole open to peer?
    hole_open: bool,
    /// Route to peer node's puncher
    peer_route: Option<Route>,
    /// Timestamp of most recent message received from peer
    peer_received_at: Instant,
    /// Option for our handle [`UdpHolePuncher`](crate::hole_puncher::UdpHolePuncher)
    /// to receive a callback when we next open a hole to peer
    wait_for_hole_open_addr: Option<Address>,
}

impl UdpHolePunchWorker {
    /// Update the Rendezvous service
    async fn rendezvous_update(&self, ctx: &mut Context) -> Result<()> {
        let msg = RendezvousRequest::Update {
            puncher_name: self.this_puncher_name.clone(),
        };
        ctx.send(self.rendezvous_route.clone(), msg).await
    }

    /// Query the Rendezvous service
    async fn rendezvous_query(&self, ctx: &mut Context) -> Result<Route> {
        let msg = RendezvousRequest::Query {
            puncher_name: self.peer_puncher_name.clone(),
        };

        // Send from a temporary context/address, so we can process the reply here
        let res = ctx
            .send_and_receive_extended::<RendezvousResponse>(
                self.rendezvous_route.clone(),
                msg,
                MessageSendReceiveOptions::new().with_timeout(QUICK_TIMEOUT),
            )
            .await?
            .body();

        match res {
            RendezvousResponse::Query(r) => r,
            _ => Err(PunchError::Internal.into()),
        }
    }

    /// Test to see if we can reach the Rendezvous service
    pub(crate) async fn rendezvous_reachable(ctx: &mut Context, rendezvous_route: &Route) -> bool {
        for _ in 0..PING_TRIES {
            trace!("Start attempt to check Rendezvous service reachability");
            let res: Result<Routed<RendezvousResponse>> = ctx
                .send_and_receive_extended(
                    rendezvous_route.clone(),
                    RendezvousRequest::Ping,
                    MessageSendReceiveOptions::new().with_timeout(QUICK_TIMEOUT),
                )
                .await;

            // Check response. Ignore all but success.
            if let Ok(msg) = res {
                if let RendezvousResponse::Pong = msg.body() {
                    trace!("Success reaching Rendezvous service");
                    return true;
                };
            }
        }
        trace!("Failed to reach Rendezvous service");
        false
    }

    pub(crate) async fn create(
        ctx: &Context,
        handle_addr: &Address,
        rendezvous_route: Route,
        this_puncher_name: &str,
        peer_puncher_name: &str,
    ) -> Result<(Address, Address)> {
        // Create worker' addresses, heartbeat & mailboxes
        let main_addr =
            Address::random_tagged(format!("UdpHolePuncher.main.{}", this_puncher_name).as_str());
        let local_addr =
            Address::random_tagged(format!("UdpHolePuncher.local.{}", this_puncher_name).as_str());

        let heartbeat =
            DelayedEvent::create(ctx, main_addr.clone(), PunchMessage::Heartbeat).await?;

        // TODO: Roll implementations of IncomingAccessControl and OutgoingAccessControl
        // to allow messaging the heartbeat, Rendezvous service and the peer (whose address
        // may be unknown and change).

        let main_mailbox = Mailbox::new(
            main_addr.clone(),
            Arc::new(AllowAll), // FIXME: @ac
            Arc::new(AllowAll), // FIXME: @ac
        );

        // TODO: Allow app to specify the access control for the local mailbox

        let local_mailbox = Mailbox::new(
            local_addr.clone(),
            Arc::new(AllowAll), // FIXME: @ac
            Arc::new(AllowAll), // FIXME: @ac
        );

        // Create and start worker
        let worker = Self {
            main_addr: main_addr.clone(),
            local_addr: local_addr.clone(),
            handle_addr: handle_addr.clone(),
            heartbeat,
            rendezvous_route,
            this_puncher_name: String::from(this_puncher_name),
            peer_puncher_name: String::from(peer_puncher_name),
            hole_open: false,
            peer_route: None,
            peer_received_at: Instant::now(),
            wait_for_hole_open_addr: None,
        };
        WorkerBuilder::new(worker)
            .with_mailboxes(Mailboxes::new(main_mailbox, vec![local_mailbox]))
            .start(ctx)
            .await?;

        Ok((main_addr, local_addr))
    }

    /// Update state to show the hole to peer is now open
    async fn set_hole_open(&mut self, ctx: &Context) -> Result<()> {
        self.hole_open = true;

        // Inform handle, if needed
        let addr = self.wait_for_hole_open_addr.take();
        if let Some(a) = addr {
            trace!("Informing handle of hole opened to peer");
            ctx.send(a, ()).await?;
        }
        Ok(())
    }

    /// Handle messages from peer
    async fn handle_peer(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Any>,
        return_route: &Route,
    ) -> Result<()> {
        debug!(
            "Peer => Puncher: {:?}, {:?}",
            PunchMessage::decode(msg.payload())?,
            msg.local_message(),
        );

        // Record contact with peer
        self.peer_received_at = Instant::now();

        // Handle message
        let inner_msg = PunchMessage::decode(msg.payload())?;
        match inner_msg {
            PunchMessage::Ping => {
                trace!("Received Ping from peer. Will Pong.");
                ctx.send(return_route.clone(), PunchMessage::Pong).await?;
            }
            PunchMessage::Pong => {
                trace!("Received Pong from peer. Setting as hole is open");
                self.set_hole_open(ctx).await?;
            }
            PunchMessage::Payload(data) => {
                trace!("Received Payload from peer. Will forward to local entity");

                // Update routing & payload
                let mut msg = msg.into_transport_message();
                msg.onward_route.step()?;
                msg.return_route.modify().prepend(self.local_addr.clone());
                msg.payload = data;

                // Forward
                debug!("Puncher => App: {:?}", msg);
                ctx.forward(LocalMessage::new(msg, vec![])).await?;
            }
            _ => return Err(PunchError::Internal.into()),
        }
        Ok(())
    }

    /// Handle heartbeat messages
    async fn handle_heartbeat(&mut self, ctx: &mut Context) -> Result<()> {
        debug!(
            "Heartbeat => Puncher: hole_open = {:?}, peer_route = {:?}",
            self.hole_open, self.peer_route
        );

        // Schedule next heartbeat here in case something below errors
        self.heartbeat.schedule(HEARTBEAT_EVERY).await?;

        // If we have not heard from peer for a while, consider hole as closed
        if self.hole_open && self.peer_received_at.elapsed() >= HOLE_OPEN_TIMEOUT {
            trace!("Not heard from peer for a while. Setting as hole closed.",);
            self.hole_open = false;
        }

        if !self.hole_open {
            // Attempt hole open if it is closed
            trace!("Hole closed. Will attempt to open hole to peer");

            // Update Rendezvous service
            self.rendezvous_update(ctx).await?;

            // Query Rendezvous service
            if let Ok(peer_route) = self.rendezvous_query(ctx).await {
                self.peer_route = Some(peer_route.clone());

                // Ping peer
                ctx.send(peer_route.clone(), PunchMessage::Ping).await?;
            }
        } else {
            // Do keepalive pings to try and keep the hole open
            if let Some(peer_route) = self.peer_route.as_ref() {
                trace!("Pinging peer for keepalive");
                ctx.send(peer_route.clone(), PunchMessage::Ping).await?;
            }
        }

        Ok(())
    }

    /// Handle messages from local entities
    async fn handle_local(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        debug!("Local => Puncher: {:?}", msg);

        if let Some(peer_route) = self.peer_route.as_ref() {
            let mut msg = msg.into_transport_message();
            debug!("App => Puncher: {:?}", msg);

            // Update routing
            msg.onward_route.step()?;
            if !msg.onward_route.contains_route(peer_route)? {
                msg.onward_route.modify().prepend_route(peer_route.clone());
            }
            msg.return_route.modify().prepend(self.main_addr.clone());

            // Wrap payload
            msg.payload = PunchMessage::Payload(msg.payload).encode()?;

            // Forward
            debug!("Puncher => Peer: {:?}", msg);
            ctx.forward(LocalMessage::new(msg, vec![])).await
        } else {
            Err(PunchError::HoleNotOpen.into())
        }
    }
}

#[ockam_core::worker]
impl Worker for UdpHolePunchWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.heartbeat.schedule(Duration::ZERO).await
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.heartbeat.cancel();
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        match msg.msg_addr() {
            // 'main' mailbox
            addr if addr == self.main_addr => {
                let return_route = msg.return_route();
                let sender_addr = msg.sender()?;
                let is_from_peer = match &self.peer_route {
                    Some(r) => return_route.contains_route(r)?,
                    None => false,
                };

                // Handle message depending on if it's from peer,
                // heartbeat or our handle
                if is_from_peer {
                    self.handle_peer(ctx, msg, &return_route).await?;
                } else if sender_addr == self.heartbeat.address() {
                    self.handle_heartbeat(ctx).await?;
                } else if sender_addr == self.handle_addr {
                    let inner_msg = PunchMessage::decode(msg.payload())?;
                    match inner_msg {
                        PunchMessage::WaitForHoleOpen => {
                            self.wait_for_hole_open_addr = Some(sender_addr)
                            // TODO: If the hole is already open, the handle
                            // will not be informed until it's closed and
                            // opened again. Is this what we want?
                        }
                        _ => return Err(PunchError::Internal.into()),
                    }
                }
            }

            // 'local' mailbox
            addr if addr == self.local_addr => {
                self.handle_local(ctx, msg).await?;
            }

            _ => return Err(PunchError::Internal.into()),
        };

        Ok(())
    }
}
