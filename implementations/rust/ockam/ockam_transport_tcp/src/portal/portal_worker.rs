use crate::portal::addresses::{Addresses, PortalType};
use crate::portal::portal_worker::ReadHalfMaybeTls::{ReadHalfNoTls, ReadHalfWithTls};
use crate::portal::portal_worker::WriteHalfMaybeTls::{WriteHalfNoTls, WriteHalfWithTls};
use crate::transport::{connect, connect_tls};
use crate::{portal::TcpPortalRecvProcessor, PortalInternalMessage, PortalMessage, TcpRegistry};
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{
    async_trait, AllowOnwardAddress, AllowSourceAddress, Decodable, DenyAll, IncomingAccessControl,
    LocalInfoIdentifier, Mailbox, Mailboxes, OutgoingAccessControl, SecureChannelLocalInfo,
};
use ockam_core::{Any, Result, Route, Routed, Worker};
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::{HostnamePort, TransportError};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;
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
    their_identifier: Option<LocalInfoIdentifier>,
    write_half: Option<WriteHalfMaybeTls>,
    read_half: Option<ReadHalfMaybeTls>,
    hostname_port: HostnamePort,
    addresses: Addresses,
    remote_route: Option<Route>,
    is_disconnecting: bool,
    portal_type: PortalType,
    last_received_packet_counter: u16,
    outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    is_tls: bool,
}

pub(crate) enum ReadHalfMaybeTls {
    ReadHalfNoTls(OwnedReadHalf),
    ReadHalfWithTls(ReadHalf<TlsStream<TcpStream>>),
}

pub(crate) enum WriteHalfMaybeTls {
    WriteHalfNoTls(OwnedWriteHalf),
    WriteHalfWithTls(WriteHalf<TlsStream<TcpStream>>),
}

impl TcpPortalWorker {
    /// Start a new `TcpPortalWorker` of type [`TypeName::Inlet`]
    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn start_new_inlet(
        ctx: &Context,
        registry: TcpRegistry,
        streams: (ReadHalfMaybeTls, WriteHalfMaybeTls),
        hostname_port: HostnamePort,
        ping_route: Route,
        their_identifier: Option<LocalInfoIdentifier>,
        addresses: Addresses,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>, // To propagate to the receiver
    ) -> Result<()> {
        Self::start(
            ctx,
            registry,
            hostname_port,
            false,
            State::SendPing { ping_route },
            their_identifier,
            Some(streams),
            addresses,
            incoming_access_control,
            outgoing_access_control,
        )
        .await
    }

    /// Start a new `TcpPortalWorker` of type [`TypeName::Outlet`]
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub(super) async fn start_new_outlet(
        ctx: &Context,
        registry: TcpRegistry,
        hostname_port: HostnamePort,
        tls: bool,
        pong_route: Route,
        their_identifier: Option<LocalInfoIdentifier>,
        addresses: Addresses,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<()> {
        Self::start(
            ctx,
            registry,
            hostname_port,
            tls,
            State::SendPong { pong_route },
            their_identifier,
            None,
            addresses,
            incoming_access_control,
            outgoing_access_control,
        )
        .await
    }

    /// Start a new `TcpPortalWorker`
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        hostname_port: HostnamePort,
        is_tls: bool,
        state: State,
        their_identifier: Option<LocalInfoIdentifier>,
        streams: Option<(ReadHalfMaybeTls, WriteHalfMaybeTls)>,
        addresses: Addresses,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<()> {
        let portal_type = if streams.is_some() {
            PortalType::Inlet
        } else {
            PortalType::Outlet
        };
        info!(
            "Creating new {:?} at sender remote: {}",
            portal_type.str(),
            addresses.sender_remote
        );

        let (rx, tx) = match streams {
            // A TcpStream is provided in case of an inlet
            Some((rx, tx)) => {
                debug!("Connected to {}", &hostname_port);
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };
        debug!("The {} supports TLS: {}", portal_type.str(), is_tls);

        let worker = Self {
            registry,
            state,
            their_identifier,
            write_half: tx,
            read_half: rx,
            hostname_port,
            addresses: addresses.clone(),
            remote_route: None,
            is_disconnecting: false,
            portal_type,
            last_received_packet_counter: u16::MAX,
            is_tls,
            outgoing_access_control: outgoing_access_control.clone(),
        };

        let internal_mailbox = Mailbox::new(
            addresses.sender_internal,
            Arc::new(AllowSourceAddress(addresses.receiver_internal)),
            Arc::new(DenyAll),
        );

        let remote_mailbox = Mailbox::new(
            addresses.sender_remote,
            incoming_access_control,
            outgoing_access_control,
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
    InvalidCounter,
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
            match rx {
                ReadHalfNoTls(rx) => self.start_receive_processor(ctx, onward_route, rx).await,
                ReadHalfWithTls(rx) => self.start_receive_processor(ctx, onward_route, rx).await,
            }
        } else {
            Err(TransportError::PortalInvalidState)?
        }
    }

    /// Start a TcpPortalRecvProcessor using a specific AsyncRead implementation (either supporting TLS or not)
    async fn start_receive_processor<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &mut self,
        ctx: &Context,
        onward_route: Route,
        rx: R,
    ) -> Result<()> {
        let receiver = TcpPortalRecvProcessor::new(
            self.registry.clone(),
            rx,
            self.addresses.clone(),
            onward_route,
        );

        let remote = Mailbox::new(
            self.addresses.receiver_remote.clone(),
            Arc::new(DenyAll),
            self.outgoing_access_control.clone(),
        );

        let internal = Mailbox::new(
            self.addresses.receiver_internal.clone(),
            Arc::new(DenyAll),
            Arc::new(AllowOnwardAddress(self.addresses.sender_internal.clone())),
        );

        ProcessorBuilder::new(receiver)
            .with_mailboxes(Mailboxes::new(remote, vec![internal]))
            .start(ctx)
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn notify_remote_about_disconnection(&mut self, ctx: &Context) -> Result<()> {
        // Notify the other end
        if let Some(remote_route) = self.remote_route.take() {
            ctx.send_from_address(
                remote_route,
                PortalMessage::Disconnect.to_neutral_message()?,
                self.addresses.sender_remote.clone(),
            )
            .await?;

            debug!(
                "Notified the other side from {:?} at: {} about connection drop",
                self.portal_type.str(),
                self.addresses.sender_internal
            );
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn stop_receiver(&self, ctx: &Context) -> Result<()> {
        if ctx
            .stop_processor(self.addresses.receiver_remote.clone())
            .await
            .is_ok()
        {
            debug!(
                "{:?} at: {} stopped receiver due to connection drop",
                self.portal_type.str(),
                self.addresses.sender_internal
            );
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn stop_sender(&self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.addresses.sender_internal.clone())
            .await
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
            // We couldn't send data to the tcp connection, let's notify the other end about dropped
            // connection and shut down both processor and worker
            DisconnectionReason::FailedTx => {
                self.notify_remote_about_disconnection(ctx).await?;
                self.stop_receiver(ctx).await?;
                // Sleep, so that if connection is dropped on both sides at the same time, the other
                // side had time to notify us about the closure. Otherwise, the message won't be
                // delivered which can lead to a warning message from a secure channel (or whatever
                // is used to deliver the message). Can be removed though
                ctx.sleep(Duration::from_secs(2)).await;
                self.stop_sender(ctx).await?;
            }
            // Packets were dropped while traveling to us, let's notify the other end about dropped
            // connection and
            DisconnectionReason::InvalidCounter => {
                self.notify_remote_about_disconnection(ctx).await?;
                self.stop_receiver(ctx).await?;
                self.stop_sender(ctx).await?;
            }
            // We couldn't read data from the tcp connection
            // Receiver should have already notified the other end and should shut down itself
            DisconnectionReason::FailedRx => {
                // Sleep, so that if connection is dropped on both sides at the same time, the other
                // side had time to notify us about the closure. Otherwise, the message won't be
                // delivered which can lead to a warning message from a secure channel (or whatever
                // is used to deliver the message). Can be removed though
                ctx.sleep(Duration::from_secs(2)).await;
                self.stop_sender(ctx).await?;
            }
            // Other end notifies us that the tcp connection is dropped
            // Let's shut down both processor and worker
            DisconnectionReason::Remote => {
                self.stop_receiver(ctx).await?;
                self.stop_sender(ctx).await?;
            }
        }

        info!(
            "{:?} at: {} stopped due to connection drop",
            self.portal_type.str(),
            self.addresses.sender_internal
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_send_ping(&self, ctx: &Context, ping_route: Route) -> Result<State> {
        // Force creation of Outlet on the other side
        ctx.send_from_address(
            ping_route,
            PortalMessage::Ping.to_neutral_message()?,
            self.addresses.sender_remote.clone(),
        )
        .await?;

        debug!("Inlet at: {} sent ping", self.addresses.sender_internal);

        Ok(State::ReceivePong)
    }

    #[instrument(skip_all)]
    async fn handle_send_pong(&mut self, ctx: &Context, pong_route: Route) -> Result<State> {
        if self.write_half.is_some() {
            // Should not happen
            return Err(TransportError::PortalInvalidState)?;
        }
        if self.is_tls {
            debug!("Connect to {} via TLS", &self.hostname_port);
            let (rx, tx) = connect_tls(&self.hostname_port).await?;
            self.write_half = Some(WriteHalfWithTls(tx));
            self.read_half = Some(ReadHalfWithTls(rx));
        } else {
            debug!("Connect to {}", self.hostname_port);
            let (rx, tx) = connect(&self.hostname_port).await?;
            self.write_half = Some(WriteHalfNoTls(tx));
            self.read_half = Some(ReadHalfNoTls(rx));
        }

        // Respond to Inlet before starting the processor but
        // after the connection has been established
        // to avoid a payload being sent before the pong
        ctx.send_from_address(
            pong_route.clone(),
            PortalMessage::Pong.to_neutral_message()?,
            self.addresses.sender_remote.clone(),
        )
        .await?;

        self.start_receiver(ctx, pong_route.clone()).await?;

        debug!(
            "Outlet at: {} successfully connected",
            self.addresses.sender_internal
        );

        debug!("Outlet at: {} sent pong", self.addresses.sender_internal);

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
                return Err(TransportError::PortalInvalidState)?;
            }
        }

        self.registry
            .add_portal_worker(&self.addresses.sender_remote);

        Ok(())
    }

    #[instrument(skip_all, name = "TcpPortalWorker::shutdown")]
    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_portal_worker(&self.addresses.sender_remote);

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    #[instrument(skip_all, name = "TcpPortalWorker::handle_message")]
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if self.is_disconnecting {
            return Ok(());
        }

        let their_identifier = SecureChannelLocalInfo::find_info(msg.local_message())
            .map(|l| l.their_identifier())
            .ok();

        if their_identifier != self.their_identifier {
            return Err(TransportError::IdentifierChanged)?;
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
        let remote_packet = recipient != self.addresses.sender_internal;
        let payload = msg.into_payload();

        match state {
            State::ReceivePong => {
                if !remote_packet {
                    return Err(TransportError::PortalInvalidState)?;
                };
                if PortalMessage::decode(&payload)? != PortalMessage::Pong {
                    return Err(TransportError::Protocol)?;
                };
                self.handle_receive_pong(ctx, return_route).await
            }
            State::Initialized => {
                trace!(
                    "{:?} at: {} received {} tcp packet",
                    self.portal_type.str(),
                    self.addresses.sender_internal,
                    if remote_packet { "remote" } else { "internal " }
                );

                if remote_packet {
                    let msg = PortalMessage::decode(&payload)?;
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
                    let msg = PortalInternalMessage::decode(&payload)?;
                    if msg != PortalInternalMessage::Disconnect {
                        return Err(TransportError::Protocol)?;
                    };
                    self.handle_disconnect(ctx).await
                }
            }
            State::SendPing { .. } | State::SendPong { .. } => {
                return Err(TransportError::PortalInvalidState)?;
            }
        }
    }
}

impl TcpPortalWorker {
    #[instrument(skip_all)]
    async fn handle_receive_pong(&mut self, ctx: &Context, return_route: Route) -> Result<()> {
        self.start_receiver(ctx, return_route.clone()).await?;
        debug!("Inlet at: {} received pong", self.addresses.sender_internal);
        self.remote_route = Some(return_route);
        self.state = State::Initialized;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_disconnect(&mut self, ctx: &Context) -> Result<()> {
        info!(
            "Tcp stream was dropped for {:?} at: {}",
            self.portal_type.str(),
            self.addresses.sender_internal
        );
        self.start_disconnection(ctx, DisconnectionReason::FailedRx)
            .await
    }

    #[instrument(skip_all)]
    async fn handle_payload(
        &mut self,
        ctx: &Context,
        payload: &[u8],
        packet_counter: Option<u16>,
    ) -> Result<()> {
        // detects both missing or out of order packets
        self.check_packet_counter(ctx, packet_counter).await?;
        let tx = if let Some(tx) = &mut self.write_half {
            tx
        } else {
            return Err(TransportError::PortalInvalidState)?;
        };

        let result = match tx {
            WriteHalfNoTls(tx) => tx.write_all(payload).await,
            WriteHalfWithTls(tx) => tx.write_all(payload).await,
        };
        if let Err(err) = result {
            warn!(
                "Failed to send message to peer {} with error: {}",
                self.hostname_port, err
            );
            self.start_disconnection(ctx, DisconnectionReason::FailedTx)
                .await?;
        }

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
                self.start_disconnection(ctx, DisconnectionReason::InvalidCounter)
                    .await?;
                return Err(TransportError::RecvBadMessage)?;
            }
            self.last_received_packet_counter = packet_counter;
        };
        Ok(())
    }
}
