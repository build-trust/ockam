use crate::portal::addresses::{Addresses, PortalType};
use crate::{portal::TcpPortalRecvProcessor, PortalInternalMessage, PortalMessage, TcpRegistry};
use core::time::Duration;
use ockam_core::compat::{boxed::Box, net::SocketAddr, sync::Arc};
use ockam_core::{
    async_trait, AllowAll, AllowOnwardAddresses, AllowSourceAddress, Decodable, DenyAll,
    IncomingAccessControl, Mailbox, Mailboxes,
};
use ockam_core::{Any, Result, Route, Routed, Worker};
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::TransportError;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, info, instrument, trace, warn};

/// Enumerate all `TcpPortalWorker` states
///
/// Possible state transitions are:
///
/// `Outlet`: `SendPong` -> `Initialized`
/// `Inlet`: `SendPing` -> `ReceivePong` -> `Initialized`
#[derive(Clone)]
enum State {
    SendPing { ping_route: Route },
    SendPong { pong_route: Route },
    ReceivePong,
    Initialized,
}

/// A TCP Portal worker
///
/// A TCP Portal worker is responsible for managing the life-cycle of
/// a portal connection and is created by
/// [`TcpInletListenProcessor::process`](crate::TcpInletListenProcessor)
/// after a new connection has been accepted.
pub(crate) struct TcpPortalWorker {
    registry: TcpRegistry,
    state: State,
    write_half: Option<OwnedWriteHalf>,
    read_half: Option<OwnedReadHalf>,
    peer: SocketAddr,
    addresses: Addresses,
    remote_route: Option<Route>,
    is_disconnecting: bool,
    portal_type: PortalType,
    last_received_packet_counter: u16,
}

impl TcpPortalWorker {
    /// Start a new `TcpPortalWorker` of type [`TypeName::Inlet`]
    #[instrument(skip_all)]
    pub(super) async fn start_new_inlet(
        ctx: &Context,
        registry: TcpRegistry,
        stream: TcpStream,
        peer: SocketAddr,
        ping_route: Route,
        addresses: Addresses,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        Self::start(
            ctx,
            registry,
            peer,
            State::SendPing { ping_route },
            Some(stream),
            addresses,
            PortalType::Inlet,
            access_control,
        )
        .await
    }

    /// Start a new `TcpPortalWorker` of type [`TypeName::Outlet`]
    #[instrument(skip_all)]
    pub(super) async fn start_new_outlet(
        ctx: &Context,
        registry: TcpRegistry,
        peer: SocketAddr,
        pong_route: Route,
        addresses: Addresses,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        Self::start(
            ctx,
            registry,
            peer,
            State::SendPong { pong_route },
            None,
            addresses,
            PortalType::Outlet,
            access_control,
        )
        .await
    }

    /// Start a new `TcpPortalWorker`
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        peer: SocketAddr,
        state: State,
        stream: Option<TcpStream>,
        addresses: Addresses,
        portal_type: PortalType,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        info!(
            "Creating new {:?} at internal: {}, remote: {}",
            portal_type.str(),
            addresses.internal,
            addresses.remote
        );

        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        let worker = Self {
            registry,
            state,
            write_half: tx,
            read_half: rx,
            peer,
            addresses: addresses.clone(),
            remote_route: None,
            is_disconnecting: false,
            portal_type,
            last_received_packet_counter: u16::MAX,
        };

        let internal_mailbox = Mailbox::new(
            addresses.internal,
            Arc::new(AllowSourceAddress(addresses.receiver)),
            Arc::new(DenyAll),
        );

        let remote_mailbox = Mailbox::new(
            addresses.remote,
            access_control,
            Arc::new(AllowAll), // FIXME: @ac Allow to respond anywhere using return_route
        );

        // start worker
        WorkerBuilder::new(worker)
            .with_mailboxes(Mailboxes::new(internal_mailbox, vec![remote_mailbox]))
            .start(ctx)
            .await?;

        Ok(())
    }
}

enum DisconnectionReason {
    FailedTx,
    FailedRx,
    Remote,
}

impl TcpPortalWorker {
    fn clone_state(&self) -> State {
        self.state.clone()
    }

    /// Start a `TcpPortalRecvProcessor`
    #[instrument(skip_all)]
    async fn start_receiver(&mut self, ctx: &Context, onward_route: Route) -> Result<()> {
        if let Some(rx) = self.read_half.take() {
            let next_hop = onward_route.next()?.clone();
            let receiver = TcpPortalRecvProcessor::new(
                self.registry.clone(),
                rx,
                self.addresses.internal.clone(),
                onward_route,
            );

            ProcessorBuilder::new(receiver)
                .with_address(self.addresses.receiver.clone())
                .with_outgoing_access_control(AllowOnwardAddresses(vec![
                    next_hop,
                    self.addresses.internal.clone(),
                ])) // Only sends messages to `onward_route` and Sender
                .start(ctx)
                .await?;

            Ok(())
        } else {
            Err(TransportError::PortalInvalidState)?
        }
    }

    #[instrument(skip_all)]
    async fn notify_remote_about_disconnection(&mut self, ctx: &Context) -> Result<()> {
        // Notify the other end
        if let Some(remote_route) = self.remote_route.take() {
            ctx.send_from_address(
                remote_route,
                PortalMessage::Disconnect,
                self.addresses.remote.clone(),
            )
            .await?;

            debug!(
                "Notified the other side from {:?} at: {} about connection drop",
                self.portal_type.str(),
                self.addresses.internal
            );
        }

        // Avoiding race condition when both inlet and outlet connections
        // are dropped at the same time. In this case we want to wait for the `Disconnect`
        // message from the other side to reach our worker, before we shut it down which
        // leads to errors (destination Worker is already stopped)
        // TODO: Remove when we have better way to handle race condition
        ctx.sleep(Duration::from_secs(1)).await;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn stop_receiver(&self, ctx: &Context) -> Result<()> {
        // Avoiding race condition when both inlet and outlet connections
        // are dropped at the same time. In this case Processor may stop itself
        // while we had `Disconnect` message from the other side. Let it stop itself,
        // but recheck that by calling `stop_processor` and ignoring the error
        // TODO: Remove when we have better way to handle race condition
        ctx.sleep(Duration::from_secs(1)).await;

        if ctx
            .stop_processor(self.addresses.receiver.clone())
            .await
            .is_ok()
        {
            debug!(
                "{:?} at: {} stopped receiver due to connection drop",
                self.portal_type.str(),
                self.addresses.internal
            );
        }

        Ok(())
    }

    /// Start the portal disconnection process
    #[instrument(skip_all)]
    async fn start_disconnection(
        &mut self,
        ctx: &Context,
        reason: DisconnectionReason,
    ) -> Result<()> {
        self.is_disconnecting = true;

        match reason {
            DisconnectionReason::FailedTx => {
                self.notify_remote_about_disconnection(ctx).await?;
            }
            DisconnectionReason::FailedRx => {
                self.notify_remote_about_disconnection(ctx).await?;
                self.stop_receiver(ctx).await?;
            }
            DisconnectionReason::Remote => {
                self.stop_receiver(ctx).await?;
            }
        }

        ctx.stop_worker(self.addresses.internal.clone()).await?;

        info!(
            "{:?} at: {} stopped due to connection drop",
            self.portal_type.str(),
            self.addresses.internal
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_send_ping(&self, ctx: &Context, ping_route: Route) -> Result<State> {
        // Force creation of Outlet on the other side
        ctx.send_from_address(
            ping_route,
            PortalMessage::Ping,
            self.addresses.remote.clone(),
        )
        .await?;

        debug!("Inlet at: {} sent ping", self.addresses.internal);

        Ok(State::ReceivePong)
    }

    #[instrument(skip_all)]
    async fn handle_send_pong(&mut self, ctx: &Context, pong_route: Route) -> Result<State> {
        if self.write_half.is_none() {
            let stream = TcpStream::connect(self.peer)
                .await
                .map_err(TransportError::from)?;
            let (rx, tx) = stream.into_split();
            self.write_half = Some(tx);
            self.read_half = Some(rx);

            // Respond to Inlet before starting the processor but
            // after the connection has been established
            // to avoid a payload being sent before the pong
            ctx.send_from_address(
                pong_route.clone(),
                PortalMessage::Pong,
                self.addresses.remote.clone(),
            )
            .await?;

            self.start_receiver(ctx, pong_route.clone()).await?;

            debug!(
                "Outlet at: {} successfully connected",
                self.addresses.internal
            );
        } else {
            ctx.send_from_address(
                pong_route.clone(),
                PortalMessage::Pong,
                self.addresses.remote.clone(),
            )
            .await?;
        }

        debug!("Outlet at: {} sent pong", self.addresses.internal);

        self.remote_route = Some(pong_route);
        Ok(State::Initialized)
    }
}

#[async_trait]
impl Worker for TcpPortalWorker {
    type Context = Context;
    type Message = Any;

    #[instrument(skip_all, name = "TcpPortalWorker::initialize")]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let state = self.clone_state();

        match state {
            State::SendPing { ping_route } => {
                self.state = self.handle_send_ping(ctx, ping_route.clone()).await?;
            }
            State::SendPong { pong_route } => {
                self.state = self.handle_send_pong(ctx, pong_route.clone()).await?;
            }
            State::ReceivePong | State::Initialized { .. } => {
                return Err(TransportError::PortalInvalidState)?
            }
        }

        self.registry.add_portal_worker(&self.addresses.remote);

        Ok(())
    }

    #[instrument(skip_all, name = "TcpPortalWorker::shutdown")]
    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_portal_worker(&self.addresses.remote);

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    #[instrument(skip_all, name = "TcpPortalWorker::handle_message")]
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if self.is_disconnecting {
            return Ok(());
        }

        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        let state = self.clone_state();
        let mut onward_route = msg.onward_route();
        let recipient = onward_route.step()?;
        if onward_route.next().is_ok() {
            return Err(TransportError::UnknownRoute)?;
        }
        let return_route = msg.return_route();
        let remote_packet = recipient != self.addresses.internal;

        match state {
            State::ReceivePong => {
                if !remote_packet {
                    return Err(TransportError::PortalInvalidState)?;
                };
                if PortalMessage::decode(msg.payload())? != PortalMessage::Pong {
                    return Err(TransportError::Protocol)?;
                };
                self.handle_receive_pong(ctx, return_route).await
            }
            State::Initialized => {
                trace!(
                    "{:?} at: {} received {} tcp packet",
                    self.portal_type.str(),
                    self.addresses.internal,
                    if remote_packet { "remote" } else { "internal " }
                );

                if remote_packet {
                    let msg = PortalMessage::decode(msg.payload())?;
                    // Send to Tcp stream
                    match msg {
                        PortalMessage::Payload(payload, packet_counter) => {
                            self.handle_payload(ctx, payload, packet_counter).await
                        }
                        PortalMessage::Disconnect => {
                            self.start_disconnection(ctx, DisconnectionReason::Remote)
                                .await
                        }
                        PortalMessage::Ping | PortalMessage::Pong => {
                            return Err(TransportError::Protocol)?;
                        }
                    }
                } else {
                    let msg = PortalInternalMessage::decode(msg.payload())?;
                    if msg != PortalInternalMessage::Disconnect {
                        return Err(TransportError::Protocol)?;
                    };
                    self.handle_disconnect(ctx).await
                }
            }
            State::SendPing { .. } | State::SendPong { .. } => {
                return Err(TransportError::PortalInvalidState)?
            }
        }
    }
}

impl TcpPortalWorker {
    #[instrument(skip_all)]
    async fn handle_receive_pong(&mut self, ctx: &Context, return_route: Route) -> Result<()> {
        self.start_receiver(ctx, return_route.clone()).await?;
        debug!("Inlet at: {} received pong", self.addresses.internal);
        self.remote_route = Some(return_route);
        self.state = State::Initialized;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_disconnect(&mut self, ctx: &Context) -> Result<()> {
        info!(
            "Tcp stream was dropped for {:?} at: {}",
            self.portal_type.str(),
            self.addresses.internal
        );
        self.start_disconnection(ctx, DisconnectionReason::FailedRx)
            .await
    }

    #[instrument(skip_all)]
    async fn handle_payload(
        &mut self,
        ctx: &Context,
        payload: Vec<u8>,
        packet_counter: Option<u16>,
    ) -> Result<()> {
        // detects both missing or out of order packets
        self.check_packet_counter(ctx, packet_counter).await?;
        if let Some(tx) = &mut self.write_half {
            match tx.write_all(&payload).await {
                Ok(()) => {}
                Err(err) => {
                    warn!(
                        "Failed to send message to peer {} with error: {}",
                        self.peer, err
                    );
                    self.start_disconnection(ctx, DisconnectionReason::FailedTx)
                        .await?;
                }
            }
        } else {
            return Err(TransportError::PortalInvalidState)?;
        };
        Ok(())
    }

    #[instrument(skip_all)]
    async fn check_packet_counter(
        &mut self,
        ctx: &Context,
        packet_counter: Option<u16>,
    ) -> Result<()> {
        if let Some(packet_counter) = packet_counter {
            let expected_counter = if self.last_received_packet_counter == u16::MAX {
                0
            } else {
                self.last_received_packet_counter + 1
            };

            if packet_counter != expected_counter {
                warn!(
                    "Received packet with counter {} while expecting {}, disconnecting",
                    packet_counter, expected_counter
                );
                self.start_disconnection(ctx, DisconnectionReason::FailedRx)
                    .await?;
                return Err(TransportError::RecvBadMessage)?;
            }
            self.last_received_packet_counter = packet_counter;
        };
        Ok(())
    }
}
