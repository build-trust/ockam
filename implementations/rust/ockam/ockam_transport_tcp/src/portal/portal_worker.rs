use crate::portal::addresses::{Addresses, PortalType};
use crate::portal::outlet_listener_registry::{MapKey, OutletListenerRegistry};
use crate::portal::portal_worker::ReadHalfMaybeTls::{ReadHalfNoTls, ReadHalfWithTls};
use crate::portal::portal_worker::WriteHalfMaybeTls::{WriteHalfNoTls, WriteHalfWithTls};
use crate::transport::{connect, connect_tls};
use crate::{portal::TcpPortalRecvProcessor, PortalInternalMessage, PortalMessage, TcpRegistry};
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{
    async_trait, AllowAll, AllowOnwardAddress, AllowSourceAddress, Decodable, DenyAll,
    IncomingAccessControl, LocalInfoIdentifier, Mailbox, Mailboxes, OutgoingAccessControl,
    SecureChannelLocalInfo,
};
use ockam_core::{Any, Result, Route, Routed, Worker};
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder, WorkerShutdownPriority};
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

pub(crate) enum HandshakeMode {
    Regular,
    Skip {
        map: Option<(MapKey, OutletListenerRegistry)>,
    },
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
    last_received_packet_counter: u16,
    outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    is_tls: bool,
    portal_payload_length: usize,
    handshake_mode: HandshakeMode,
    enable_nagle: bool,
    buffer_to_ignore: Vec<u8>,
    buffer_to_send: Vec<u8>,
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
    pub(super) fn start_new_inlet(
        ctx: &Context,
        registry: TcpRegistry,
        streams: (ReadHalfMaybeTls, WriteHalfMaybeTls),
        hostname_port: HostnamePort,
        ping_route: Route,
        their_identifier: Option<LocalInfoIdentifier>,
        addresses: Addresses,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>, // To propagate to the receiver
        portal_payload_length: usize,
        skip_handshake: bool,
        buffer_to_send: Vec<u8>,
        buffer_to_ignore: Vec<u8>,
    ) -> Result<()> {
        let handshake_mode = if skip_handshake {
            HandshakeMode::Skip { map: None }
        } else {
            HandshakeMode::Regular
        };

        Self::start(
            ctx,
            registry,
            hostname_port,
            false,
            State::SendPing { ping_route },
            None,
            their_identifier,
            Some(streams),
            addresses,
            incoming_access_control,
            outgoing_access_control,
            portal_payload_length,
            handshake_mode,
            false,
            buffer_to_send,
            buffer_to_ignore,
        )
    }

    /// Start a new `TcpPortalWorker` of type [`TypeName::Outlet`]
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub(super) fn start_new_outlet(
        ctx: &Context,
        registry: TcpRegistry,
        hostname_port: HostnamePort,
        tls: bool,
        pong_route: Route,
        their_identifier: Option<LocalInfoIdentifier>,
        addresses: Addresses,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        portal_payload_length: usize,
    ) -> Result<()> {
        Self::start(
            ctx,
            registry,
            hostname_port,
            tls,
            State::SendPong { pong_route },
            None,
            their_identifier,
            None,
            addresses,
            incoming_access_control,
            outgoing_access_control,
            portal_payload_length,
            HandshakeMode::Regular,
            false,
            vec![],
            vec![],
        )
    }

    /// Start a new `TcpPortalWorker` of type [`TypeName::Outlet`]
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub(super) fn start_new_outlet_no_handshake(
        ctx: &Context,
        registry: TcpRegistry,
        hostname_port: HostnamePort,
        tls: bool,
        pong_route: Route,
        their_identifier: Option<LocalInfoIdentifier>,
        addresses: Addresses,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        portal_payload_length: usize,
        map_key: MapKey,
        outlet_listener_registry: OutletListenerRegistry,
    ) -> Result<()> {
        Self::start(
            ctx,
            registry,
            hostname_port,
            tls,
            State::Initialized,
            Some(pong_route),
            their_identifier,
            None,
            addresses,
            // We now only receive messages from the "outlet" address on our own node
            Arc::new(AllowAll),
            outgoing_access_control,
            portal_payload_length,
            HandshakeMode::Skip {
                map: Some((map_key, outlet_listener_registry)),
            },
            false,
            vec![],
            vec![],
        )
    }

    /// Start a new `TcpPortalWorker`
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    fn start(
        ctx: &Context,
        registry: TcpRegistry,
        hostname_port: HostnamePort,
        is_tls: bool,
        state: State,
        remote_route: Option<Route>,
        their_identifier: Option<LocalInfoIdentifier>,
        streams: Option<(ReadHalfMaybeTls, WriteHalfMaybeTls)>,
        addresses: Addresses,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        portal_payload_length: usize,
        handshake_mode: HandshakeMode,
        enable_nagle: bool,
        buffer_to_send: Vec<u8>,
        buffer_to_ignore: Vec<u8>,
    ) -> Result<()> {
        debug!(%addresses.portal_type, sender_remote=%addresses.sender_remote, %is_tls, "creating portal worker");

        let (rx, tx) = match streams {
            // A TcpStream is provided in case of an inlet
            Some((rx, tx)) => {
                debug!("Connected to {}", &hostname_port);
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        let worker = Self {
            registry,
            state,
            their_identifier,
            write_half: tx,
            read_half: rx,
            hostname_port,
            addresses: addresses.clone(),
            remote_route,
            is_disconnecting: false,
            last_received_packet_counter: u16::MAX,
            is_tls,
            outgoing_access_control: outgoing_access_control.clone(),
            portal_payload_length,
            enable_nagle,
            handshake_mode,
            buffer_to_send,
            buffer_to_ignore,
        };

        let internal_mailbox = Mailbox::new(
            addresses.sender_internal,
            None,
            Arc::new(AllowSourceAddress(addresses.receiver_internal)),
            Arc::new(DenyAll),
        );

        let remote_mailbox = Mailbox::new(
            addresses.sender_remote,
            None,
            incoming_access_control,
            outgoing_access_control,
        );

        // start worker
        WorkerBuilder::new(worker)
            .with_mailboxes(Mailboxes::new(internal_mailbox, vec![remote_mailbox]))
            .with_shutdown_priority(WorkerShutdownPriority::Priority4)
            .start(ctx)?;

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
    fn skip_handshake(&self) -> bool {
        match &self.handshake_mode {
            HandshakeMode::Regular => false,
            HandshakeMode::Skip { .. } => true,
        }
    }

    /// Start a `TcpPortalRecvProcessor`
    #[instrument(skip_all)]
    fn start_receiver(&mut self, ctx: &Context, onward_route: Route) -> Result<()> {
        if let Some(rx) = self.read_half.take() {
            match rx {
                ReadHalfNoTls(rx) => {
                    self.start_receive_processor(ctx, onward_route, rx, self.buffer_to_send.clone())
                }
                ReadHalfWithTls(rx) => {
                    self.start_receive_processor(ctx, onward_route, rx, self.buffer_to_send.clone())
                }
            }
        } else {
            Err(TransportError::PortalInvalidState)?
        }
    }

    /// Start a TcpPortalRecvProcessor using a specific AsyncRead implementation (either supporting TLS or not)
    fn start_receive_processor<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &mut self,
        ctx: &Context,
        onward_route: Route,
        rx: R,
        buffer_to_send: Vec<u8>,
    ) -> Result<()> {
        let receiver = TcpPortalRecvProcessor::new(
            self.registry.clone(),
            rx,
            self.addresses.clone(),
            onward_route,
            self.portal_payload_length,
            buffer_to_send,
        );

        let remote = Mailbox::new(
            self.addresses.receiver_remote.clone(),
            None,
            Arc::new(DenyAll),
            self.outgoing_access_control.clone(),
        );

        let internal = Mailbox::new(
            self.addresses.receiver_internal.clone(),
            None,
            Arc::new(DenyAll),
            Arc::new(AllowOnwardAddress(self.addresses.sender_internal.clone())),
        );

        ProcessorBuilder::new(receiver)
            .with_mailboxes(Mailboxes::new(remote, vec![internal]))
            .with_shutdown_priority(WorkerShutdownPriority::Priority3)
            .start(ctx)?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn notify_remote_about_disconnection(&mut self, ctx: &Context) {
        // Notify the other end
        let remote_route = if let Some(remote_route) = self.remote_route.take() {
            remote_route
        } else {
            return;
        };

        let disconnect_msg = match PortalMessage::Disconnect.to_neutral_message() {
            Ok(msg) => msg,
            Err(_) => return,
        };

        if ctx
            .send_from_address(
                remote_route,
                disconnect_msg,
                self.addresses.sender_remote.clone(),
            )
            .await
            .is_err()
        {
            debug!(
                portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
                "error notifying the other side of portal that the connection is dropped",
            );
        } else {
            debug!(
                portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
                "notified the other side of portal that the connection is dropped",
            );
        }
    }

    #[instrument(skip_all)]
    fn stop_receiver(&self, ctx: &Context) {
        match ctx.stop_address(&self.addresses.receiver_remote) {
            Ok(_) => {
                debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
                "stopped receiver due to connection drop");
            }
            Err(_) => {
                debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
                "error stopping receiver due to connection drop");
            }
        }
    }

    #[instrument(skip_all)]
    fn stop_sender(&self, ctx: &Context) -> Result<()> {
        ctx.stop_address(&self.addresses.sender_internal)
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
                self.notify_remote_about_disconnection(ctx).await;
                self.stop_receiver(ctx);
                // Sleep, so that if connection is dropped on both sides at the same time, the other
                // side had time to notify us about the closure. Otherwise, the message won't be
                // delivered which can lead to a warning message from a secure channel (or whatever
                // is used to deliver the message). Can be removed though
                ctx.sleep(Duration::from_secs(2)).await;
                self.stop_sender(ctx)?;
            }
            // Packets were dropped while traveling to us, let's notify the other end about dropped
            // connection and
            DisconnectionReason::InvalidCounter => {
                self.notify_remote_about_disconnection(ctx).await;
                self.stop_receiver(ctx);
                self.stop_sender(ctx)?;
            }
            // We couldn't read data from the tcp connection
            // Receiver should have already notified the other end and should shut down itself
            DisconnectionReason::FailedRx => {
                // Sleep, so that if connection is dropped on both sides at the same time, the other
                // side had time to notify us about the closure. Otherwise, the message won't be
                // delivered which can lead to a warning message from a secure channel (or whatever
                // is used to deliver the message). Can be removed though
                ctx.sleep(Duration::from_secs(2)).await;
                self.stop_sender(ctx)?;
            }
            // Other end notifies us that the tcp connection is dropped
            // Let's shut down both processor and worker
            DisconnectionReason::Remote => {
                self.stop_receiver(ctx);
                self.stop_sender(ctx)?;
            }
        }

        debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
            "stopped due to connection drop");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_send_ping(&mut self, ctx: &Context, ping_route: Route) -> Result<State> {
        // Force creation of Outlet on the other side
        ctx.send_from_address(
            ping_route.clone(),
            PortalMessage::Ping.to_neutral_message()?,
            self.addresses.sender_remote.clone(),
        )
        .await?;

        debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal, "sent ping");

        if self.skip_handshake() {
            self.remote_route = Some(ping_route.clone());
            self.start_receiver(ctx, ping_route)?;

            Ok(State::Initialized)
        } else {
            Ok(State::ReceivePong)
        }
    }

    async fn connect(&mut self) -> Result<()> {
        if self.is_tls {
            debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal, "connect to {} via TLS", &self.hostname_port);
            let (rx, tx) = connect_tls(&self.hostname_port, self.enable_nagle).await?;
            self.write_half = Some(WriteHalfWithTls(tx));
            self.read_half = Some(ReadHalfWithTls(rx));
        } else {
            debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal, "connect to {}", self.hostname_port);
            let (rx, tx) = connect(&self.hostname_port, self.enable_nagle, None).await?;
            self.write_half = Some(WriteHalfNoTls(tx));
            self.read_half = Some(ReadHalfNoTls(rx));
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_send_pong(&mut self, ctx: &Context, pong_route: Route) -> Result<State> {
        if self.write_half.is_some() {
            // Should not happen
            return Err(TransportError::PortalInvalidState)?;
        }

        self.connect().await?;

        // Respond to Inlet before starting the processor but
        // after the connection has been established
        // to avoid a payload being sent before the pong
        ctx.send_from_address(
            pong_route.clone(),
            PortalMessage::Pong.to_neutral_message()?,
            self.addresses.sender_remote.clone(),
        )
        .await?;

        self.start_receiver(ctx, pong_route.clone())?;

        debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal, "sent pong");

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
        match &self.state {
            State::SendPing { ping_route } => {
                self.state = self.handle_send_ping(ctx, ping_route.clone()).await?;
            }
            State::SendPong { pong_route } => {
                self.state = self.handle_send_pong(ctx, pong_route.clone()).await?;
            }
            State::Initialized => {
                self.connect().await?;
                self.start_receiver(ctx, self.remote_route.clone().unwrap())?;
            }
            State::ReceivePong => {
                return Err(TransportError::PortalInvalidState)?;
            }
        }

        self.registry
            .add_portal_worker(&self.addresses.sender_remote);

        let (portal_type, listener_address) = match &self.addresses.portal_type {
            PortalType::Inlet { listener_address }
            | PortalType::PrivilegedInlet { listener_address } => ("TCP Inlet", listener_address),
            PortalType::Outlet | PortalType::PrivilegedOutlet => {
                ("TCP Outlet", &self.hostname_port)
            }
        };
        info! {
            target: "ockam_command::user",
            "The {} listening at {} connected to {}",
            portal_type,
            listener_address,
            self.their_identifier
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        }

        debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
            "tcp portal worker initialized"
        );

        Ok(())
    }

    #[instrument(skip_all, name = "TcpPortalWorker::shutdown")]
    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        if let HandshakeMode::Skip { map } = &mut self.handshake_mode {
            if let Some((map_key, outlet_listener_registry)) = map.take() {
                outlet_listener_registry
                    .started_workers
                    .write()
                    .unwrap()
                    .remove(&map_key);
            }
        }

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

        // Remove our own address from the route so the other end
        // knows what to do with the incoming message

        let msg = msg.into_local_message();
        let mut onward_route = msg.onward_route;
        let recipient = onward_route.step()?;
        if onward_route.next().is_ok() {
            return Err(TransportError::UnknownRoute)?;
        }

        let remote_packet = recipient != self.addresses.sender_internal;
        if remote_packet {
            let their_identifier = SecureChannelLocalInfo::find_info_from_list(&msg.local_info)
                .map(|l| l.their_identifier())
                .ok();

            if their_identifier != self.their_identifier {
                debug!(
                    "identifier changed from {:?} to {:?}",
                    self.their_identifier.as_ref().map(|i| i.to_string()),
                    their_identifier.as_ref().map(|i| i.to_string()),
                );
                return Err(TransportError::IdentifierChanged)?;
            }
        }

        let return_route = msg.return_route;
        let payload = msg.payload;

        match &self.state {
            State::ReceivePong => {
                if !remote_packet {
                    return Err(TransportError::PortalInvalidState)?;
                };
                if PortalMessage::decode(&payload)? != PortalMessage::Pong {
                    return Err(TransportError::Protocol)?;
                };
                self.handle_receive_pong(ctx, return_route)
            }
            State::Initialized => {
                trace!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
                    "received {} tcp packet",
                    if remote_packet { "remote" } else { "internal " },
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
                        PortalMessage::Ping | PortalMessage::Pong => Ok(()),
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
                Err(TransportError::PortalInvalidState)?
            }
        }
    }
}

impl TcpPortalWorker {
    #[instrument(skip_all)]
    fn handle_receive_pong(&mut self, ctx: &Context, return_route: Route) -> Result<()> {
        self.start_receiver(ctx, return_route.clone())?;
        debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal, "received pong");
        self.remote_route = Some(return_route);
        self.state = State::Initialized;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_disconnect(&mut self, ctx: &Context) -> Result<()> {
        let (portal_type, listener_address) = match &self.addresses.portal_type {
            PortalType::Inlet { listener_address }
            | PortalType::PrivilegedInlet { listener_address } => ("TCP Inlet", listener_address),
            PortalType::Outlet | PortalType::PrivilegedOutlet => {
                ("TCP Outlet", &self.hostname_port)
            }
        };
        info! {
            target: "ockam_command::user",
            "The {} listening at {} was disconnected from {}",
            portal_type,
            listener_address,
            self.their_identifier
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        }
        debug!(portal_type = %self.addresses.portal_type, sender_internal = %self.addresses.sender_internal,
            "tcp stream was dropped");
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

        info!("payload size before: {}", payload.len());

        let payload = if self.buffer_to_ignore.is_empty() {
            info!("No buffer to ignore");
            payload
        } else {
            let bytes_to_verify_and_ignore = self.buffer_to_ignore.len();
            info!("Ignoring {} bytes", bytes_to_verify_and_ignore);
            if payload.len() >= bytes_to_verify_and_ignore {
                info!("Received packet with buffer to ignore, ignoring");
                let to_verify = &payload[..bytes_to_verify_and_ignore];
                if self.buffer_to_ignore != to_verify {
                    warn!(portal_type = %self.addresses.portal_type,
                        "Received invalid packet, disconnecting"
                    );
                    self.start_disconnection(ctx, DisconnectionReason::InvalidCounter)
                        .await?;
                    return Err(TransportError::RecvBadMessage)?;
                }
                self.buffer_to_ignore.clear();
                &payload[bytes_to_verify_and_ignore..]
            } else {
                todo!("shouldn't be neede yet")
            }
        };

        info!("payload size after: {}", payload.len());
        if payload.is_empty() {
            return Ok(());
        }

        let result = match tx {
            WriteHalfNoTls(tx) => tx.write_all(payload).await,
            WriteHalfWithTls(tx) => tx.write_all(payload).await,
        };
        if let Err(err) = result {
            warn!(portal_type = %self.addresses.portal_type, %err,
                "failed to send message to peer {} with error",
                self.hostname_port
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
                warn!(portal_type = %self.addresses.portal_type,
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
