use crate::{TcpRecvProcessor, TcpRouterHandle};
use core::time::Duration;
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    AllowSourceAddress, DenyAll, LocalOnwardOnly,
};
use ockam_core::{
    Address, Any, Decodable, Encodable, LocalMessage, Mailbox, Mailboxes, Message, Result, Routed,
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

/// Provides the transmit and receive parts of a TCP connection
#[derive(Debug)]
pub(crate) struct WorkerPair {
    hostnames: Vec<String>,
    peer: SocketAddr,
    tx_addr: Address,
}

impl WorkerPair {
    /// Return a reference to the peer's hostname(s)
    pub fn hostnames(&self) -> &[String] {
        &self.hostnames
    }

    /// Return a reference to the peer's [`SocketAddr`](std::net::SocketAddr)
    pub fn peer(&self) -> SocketAddr {
        self.peer
    }

    /// Return a clone of the transmit [`Address`]
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

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
    router_handle: TcpRouterHandle,
    rx: Option<OwnedReadHalf>,
    tx: Option<OwnedWriteHalf>,
    peer: SocketAddr,
    internal_addr: Address,
    rx_addr: Address,
    rx_should_be_stopped: bool,
}

impl TcpSendWorker {
    /// Create a new `TcpSendWorker`
    fn new(
        router_handle: TcpRouterHandle,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        internal_addr: Address,
        rx_addr: Address,
    ) -> Self {
        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        Self {
            router_handle,
            rx,
            tx,
            peer,
            internal_addr,
            rx_addr,
            rx_should_be_stopped: true,
        }
    }

    pub(crate) fn internal_addr(&self) -> &Address {
        &self.internal_addr
    }

    // Receiver processor address
    pub(crate) fn rx_addr(&self) -> &Address {
        &self.rx_addr
    }

    /// Create a `(TcpSendWorker, WorkerPair)` without spawning the worker.
    pub(crate) async fn new_pair(
        router_handle: TcpRouterHandle,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<(Self, WorkerPair)> {
        let role_str = if stream.is_none() {
            "initiator"
        } else {
            "responder"
        };

        let tx_addr = Address::random_tagged(&format!("TcpSendWorker_tx_addr_{}", role_str));
        let int_addr = Address::random_tagged(&format!("TcpSendWorker_int_addr_{}", role_str));
        let rx_addr = Address::random_tagged(&format!("TcpRecvProcessor_{}", role_str));
        let sender = TcpSendWorker::new(router_handle, stream, peer, int_addr, rx_addr);
        Ok((
            sender,
            WorkerPair {
                hostnames,
                peer,
                tx_addr,
            },
        ))
    }

    /// Start a `(TcpSendWorker, TcpRecvProcessor)` pair that opens and
    /// manages the connection with the given peer
    pub(crate) async fn start_pair(
        ctx: &Context,
        router_handle: TcpRouterHandle,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        let tcprouter_main_addr = router_handle.main_addr().clone();

        trace!("Creating new TCP worker pair");
        let (sender, pair) = Self::new_pair(router_handle, stream, peer, hostnames).await?;

        // Allow messages routed from Tcp Router
        let tx_mailbox = Mailbox::new(
            pair.tx_addr(),
            Arc::new(AllowSourceAddress(tcprouter_main_addr)),
            Arc::new(DenyAll),
        );

        let internal_mailbox = Mailbox::new(
            sender.internal_addr().clone(),
            Arc::new(AllowSourceAddress(sender.rx_addr().clone())),
            Arc::new(DenyAll),
        );

        WorkerBuilder::with_mailboxes(Mailboxes::new(tx_mailbox, vec![internal_mailbox]), sender)
            .start(ctx)
            .await?;

        Ok(pair)
    }

    async fn stop_and_unregister(&self, ctx: &Context) -> Result<()> {
        self.router_handle.unregister(ctx.address()).await?;

        ctx.stop_worker(ctx.address()).await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        if self.tx.is_none() {
            debug!(addr = %self.peer, "Connecting");
            let connection = match TcpStream::connect(self.peer).await {
                Ok(c) => {
                    debug!(addr = %self.peer, "Connected");
                    c
                }
                Err(e) => {
                    debug!(addr = %self.peer, err = %e, "Failed to connect");
                    self.stop_and_unregister(ctx).await?;

                    return Err(TransportError::from(e).into());
                }
            };

            let keepalive = TcpKeepalive::new()
                .with_time(Duration::from_secs(300))
                .with_retries(2)
                .with_interval(Duration::from_secs(75));
            let socket = SockRef::from(&connection);
            socket.set_tcp_keepalive(&keepalive).unwrap();

            let (rx, tx) = connection.into_split();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }

        let rx = self.rx.take().ok_or(TransportError::GenericIo)?;

        let receiver = TcpRecvProcessor::new(
            rx,
            format!("{}#{}", crate::TCP, self.peer).into(),
            self.internal_addr.clone(),
        );

        let mailbox = Mailbox::new(
            self.rx_addr().clone(),
            Arc::new(DenyAll),
            Arc::new(LocalOnwardOnly), // Sending to this sender and workers that can receive messages from TCP
        );
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), receiver)
            .start(ctx)
            .await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if self.rx_should_be_stopped {
            let _ = ctx.stop_processor(self.rx_addr().clone()).await;
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
        let tx = match &mut self.tx {
            Some(tx) => tx,
            None => return Err(TransportError::PeerNotFound.into()),
        };

        let recipient = msg.msg_addr();
        if recipient == self.internal_addr {
            let msg = TcpSendWorkerMsg::decode(msg.payload())?;

            match msg {
                TcpSendWorkerMsg::ConnectionClosed => {
                    info!("Stopping sender due to closed connection {}", self.peer);
                    // No need to stop Receiver as it notified us about connection drop and will
                    // stop itself
                    self.rx_should_be_stopped = false;
                    self.stop_and_unregister(ctx).await?;

                    return Ok(());
                }
            }
        } else {
            let mut msg = LocalMessage::decode(msg.payload())?.into_transport_message();
            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            msg.onward_route.step()?;
            // Create a message buffer with prepended length
            let msg = prepare_message(msg)?;

            if tx.write_all(msg.as_slice()).await.is_err() {
                warn!("Failed to send message to peer {}", self.peer);
                self.stop_and_unregister(ctx).await?;

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
