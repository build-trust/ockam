use crate::{TcpRecvProcessor, TcpRegistry};
use cfg_if::cfg_if;
use core::time::Duration;
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    AllowSourceAddress, DenyAll, IncomingAccessControl, OutgoingAccessControl,
};
use ockam_core::{
    Address, Any, Decodable, Encodable, Mailbox, Mailboxes, Message, Result, Routed,
    TransportMessage, Worker,
};
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::TransportError;
use serde::{Deserialize, Serialize};
use socket2::{SockRef, TcpKeepalive};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, info, trace, warn};

#[derive(Serialize, Deserialize, Message, Clone)]
pub(crate) enum TcpSendWorkerMsg {
    ConnectionClosed,
}

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`TcpSendWorker::start_pair`](crate::TcpSendWorker::start_pair)
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpSendWorker {
    registry: TcpRegistry,
    read_half: Option<OwnedReadHalf>,
    write_half: Option<OwnedWriteHalf>,
    peer: SocketAddr,
    self_internal_addr: Address,
    self_address: Address,
    receiver_processor_address: Address,
    receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    rx_should_be_stopped: bool,
}

impl TcpSendWorker {
    /// Create a new `TcpSendWorker`
    fn new(
        registry: TcpRegistry,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        self_internal_addr: Address,
        self_address: Address,
        receiver_processor_address: Address,
        receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        Self {
            registry,
            read_half: rx,
            write_half: tx,
            peer,
            self_internal_addr,
            receiver_processor_address,
            self_address,
            receiver_outgoing_access_control,
            rx_should_be_stopped: true,
        }
    }

    pub(crate) fn self_internal_addr(&self) -> &Address {
        &self.self_internal_addr
    }

    pub(crate) fn self_address(&self) -> &Address {
        &self.self_address
    }

    pub(crate) fn receiver_processor_address(&self) -> &Address {
        &self.receiver_processor_address
    }
}

impl TcpSendWorker {
    /// Create a `(TcpSendWorker, WorkerPair)` without spawning the worker.
    pub(crate) async fn create(
        registry: TcpRegistry,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<Self> {
        let role_str = if stream.is_none() {
            "initiator"
        } else {
            "responder"
        };

        let self_address = Address::random_tagged(&format!("TcpSendWorker_tx_addr_{}", role_str));
        let self_internal_address =
            Address::random_tagged(&format!("TcpSendWorker_int_addr_{}", role_str));
        let receiver_processor_address =
            Address::random_tagged(&format!("TcpRecvProcessor_{}", role_str));
        let sender = TcpSendWorker::new(
            registry,
            stream,
            peer,
            self_internal_address,
            self_address,
            receiver_processor_address,
            outgoing_access_control,
        );
        Ok(sender)
    }

    /// Create a `(TcpSendWorker, TcpRecvProcessor)` pair that opens and
    /// manages the connection with the given peer
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
        receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<Address> {
        trace!("Creating new TCP worker pair");
        let sender_worker =
            Self::create(registry, stream, peer, receiver_outgoing_access_control).await?;
        let self_address = sender_worker.self_address().clone();

        let main_mailbox = Mailbox::new(
            self_address.clone(),
            sender_incoming_access_control,
            Arc::new(DenyAll),
        );

        let internal_mailbox = Mailbox::new(
            sender_worker.self_internal_addr().clone(),
            Arc::new(AllowSourceAddress(
                sender_worker.receiver_processor_address().clone(),
            )),
            Arc::new(DenyAll),
        );

        WorkerBuilder::with_mailboxes(
            Mailboxes::new(main_mailbox, vec![internal_mailbox]),
            sender_worker,
        )
        .start(ctx)
        .await?;

        Ok(self_address)
    }

    async fn stop(&self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.self_address.clone()).await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        if self.write_half.is_none() {
            debug!(addr = %self.peer, "Connecting");
            let connection = match TcpStream::connect(self.peer).await {
                Ok(c) => {
                    debug!(addr = %self.peer, "Connected");
                    c
                }
                Err(e) => {
                    debug!(addr = %self.peer, err = %e, "Failed to connect");
                    self.stop(ctx).await?;

                    return Err(TransportError::from(e).into());
                }
            };

            let mut keepalive = TcpKeepalive::new()
                .with_time(Duration::from_secs(300))
                .with_interval(Duration::from_secs(75));

            cfg_if! {
                if #[cfg(unix)] {
                   keepalive = keepalive.with_retries(2);
                }
            }

            let socket = SockRef::from(&connection);
            socket.set_tcp_keepalive(&keepalive).unwrap();

            let (rx, tx) = connection.into_split();
            self.write_half = Some(tx);
            self.read_half = Some(rx);
        }

        let rx = self.read_half.take().ok_or(TransportError::GenericIo)?;

        let receiver = TcpRecvProcessor::new(
            self.registry.clone(),
            rx,
            self.peer.to_string(),
            self.self_address.clone(),
            self.self_internal_addr.clone(),
        );

        let mailbox = Mailbox::new(
            self.receiver_processor_address().clone(),
            Arc::new(DenyAll),
            self.receiver_outgoing_access_control.clone(),
        );
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), receiver)
            .start(ctx)
            .await?;

        self.registry.add_sender_worker(&self.self_address);

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_sender_worker(&self.self_address);

        if self.rx_should_be_stopped {
            let _ = ctx
                .stop_processor(self.receiver_processor_address().clone())
                .await;
        }

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let write_half = match &mut self.write_half {
            Some(write_half) => write_half,
            None => return Err(TransportError::PeerNotFound.into()),
        };

        let recipient = msg.msg_addr();
        if recipient == self.self_internal_addr {
            let msg = TcpSendWorkerMsg::decode(msg.payload())?;

            match msg {
                TcpSendWorkerMsg::ConnectionClosed => {
                    info!("Stopping sender due to closed connection {}", self.peer);
                    // No need to stop Receiver as it notified us about connection drop and will
                    // stop itself
                    self.rx_should_be_stopped = false;
                    self.stop(ctx).await?;

                    return Ok(());
                }
            }
        } else {
            let mut msg = msg.into_transport_message();
            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            msg.onward_route.step()?;
            // Create a message buffer with prepended length
            let msg = prepare_message(msg)?;

            if write_half.write_all(msg.as_slice()).await.is_err() {
                warn!("Failed to send message to peer {}", self.peer);
                self.stop(ctx).await?;

                return Ok(());
            }
        }

        Ok(())
    }
}

/// Helper that creates a length-prefixed buffer containing the given
/// `TransportMessage`'s payload
///
/// The length-prefix is encoded as a big-endian 16-bit unsigned
/// integer.
fn prepare_message(msg: TransportMessage) -> Result<Vec<u8>> {
    let mut msg_buf = msg.encode().map_err(|_| TransportError::SendBadMessage)?;

    // Create a buffer that includes the message length in big endian
    let mut len = (msg_buf.len() as u16).to_be_bytes().to_vec();

    // Fun fact: reversing a vector in place, appending the length,
    // and then reversing it again is faster for large message sizes
    // than adding the large chunk of data.
    //
    // https://play.rust-lang.org/?version=stable&mode=release&edition=2018&gist=8669a640004ac85c7be38b19e3e73dcb
    msg_buf.reverse();
    len.reverse();
    msg_buf.append(&mut len);
    msg_buf.reverse();

    Ok(msg_buf)
}
