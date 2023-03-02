use crate::workers::Addresses;
use crate::TcpRegistry;
use cfg_if::cfg_if;
use core::time::Duration;
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    AllowSourceAddress, DenyAll, IncomingAccessControl,
};
use ockam_core::{
    Any, Decodable, Encodable, Mailbox, Mailboxes, Message, Result, Routed, TransportMessage,
    Worker,
};
use ockam_node::{Context, WorkerBuilder};
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

pub(crate) enum ConnectionRole {
    Initiator,
    Responder,
}

impl ConnectionRole {
    pub(crate) fn str(&self) -> &'static str {
        match self {
            ConnectionRole::Initiator => "initiator",
            ConnectionRole::Responder => "responder",
        }
    }
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
    write_half: OwnedWriteHalf,
    peer: SocketAddr,
    addresses: Addresses,
    rx_should_be_stopped: bool,
}

impl TcpSendWorker {
    /// Create a new `TcpSendWorker`
    fn new(
        registry: TcpRegistry,
        write_half: OwnedWriteHalf,
        peer: SocketAddr,
        addresses: Addresses,
    ) -> Self {
        Self {
            registry,
            write_half,
            peer,
            addresses,
            rx_should_be_stopped: true,
        }
    }
}

impl TcpSendWorker {
    /// Create a `(TcpSendWorker, TcpRecvProcessor)` pair that opens and
    /// manages the connection with the given peer
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        write_half: OwnedWriteHalf,
        addresses: &Addresses,
        peer: SocketAddr,
        sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        trace!("Creating new TCP worker pair");
        let sender_worker = Self::new(registry, write_half, peer, addresses.clone());

        let main_mailbox = Mailbox::new(
            addresses.sender_address().clone(),
            sender_incoming_access_control,
            Arc::new(DenyAll),
        );

        let internal_mailbox = Mailbox::new(
            addresses.sender_internal_addr().clone(),
            Arc::new(AllowSourceAddress(addresses.receiver_address().clone())),
            Arc::new(DenyAll),
        );

        WorkerBuilder::with_mailboxes(
            Mailboxes::new(main_mailbox, vec![internal_mailbox]),
            sender_worker,
        )
        .start(ctx)
        .await?;

        Ok(())
    }

    async fn stop(&self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.addresses.sender_address().clone())
            .await?;

        Ok(())
    }

    pub(crate) async fn connect(peer: SocketAddr) -> Result<(OwnedReadHalf, OwnedWriteHalf)> {
        debug!(addr = %peer, "Connecting");
        let connection = match TcpStream::connect(peer).await {
            Ok(c) => {
                debug!(addr = %peer, "Connected");
                c
            }
            Err(e) => {
                debug!(addr = %peer, err = %e, "Failed to connect");
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

        Ok(connection.into_split())
    }
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        self.registry
            .add_sender_worker(self.addresses.sender_address());

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_sender_worker(self.addresses.sender_address());

        if self.rx_should_be_stopped {
            let _ = ctx
                .stop_processor(self.addresses.receiver_address().clone())
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
        let recipient = msg.msg_addr();
        if &recipient == self.addresses.sender_internal_addr() {
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

            if self.write_half.write_all(msg.as_slice()).await.is_err() {
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
