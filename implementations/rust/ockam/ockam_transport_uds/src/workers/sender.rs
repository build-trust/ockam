use std::os::unix::net::SocketAddr;

use ockam_core::{
    async_trait, compat::sync::Arc, Address, AllowAll, Any, Decodable, Encodable, LocalMessage,
    Mailbox, Mailboxes, Message, Result, Routed, TransportMessage, Worker,
};
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::TransportError;
use serde::{Deserialize, Serialize};
use socket2::SockRef;
use tokio::{
    io::AsyncWriteExt,
    net::{
        unix::{OwnedReadHalf, OwnedWriteHalf},
        UnixStream,
    },
};
use tracing::{debug, error, trace, warn};

use crate::router::UdsRouterHandle;

use super::UdsRecvProcessor;

/// Provides the transmit and Socket Addr of a UDS connection
#[derive(Debug)]
pub(crate) struct WorkerPair {
    paths: Vec<String>,
    peer: SocketAddr,
    tx_addr: Address,
}

impl WorkerPair {
    /// Returns a reference to the peers pathnames
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// Return a reference to the peers SocketAddr
    pub fn peer(&self) -> &SocketAddr {
        &self.peer
    }

    /// Return a clone of the transmit
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

#[derive(Serialize, Deserialize, Message, Clone)]
pub(crate) enum UdsSendWorkerMsg {
    ConnectionClosed,
}

pub(crate) struct UdsSendWorker {
    router_handle: UdsRouterHandle,
    rx: Option<OwnedReadHalf>,
    tx: Option<OwnedWriteHalf>,
    peer: SocketAddr,
    internal_addr: Address,
    rx_addr: Option<Address>,
}

impl UdsSendWorker {
    /// Create a new [`UdsSendWorker`]
    fn new(
        router_handle: UdsRouterHandle,
        stream: Option<UnixStream>,
        peer: SocketAddr,
        internal_addr: Address,
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
            rx_addr: None,
        }
    }

    pub(crate) fn internal_addr(&self) -> &Address {
        &self.internal_addr
    }

    /// Create a ([`UdsSendWorker`],[`WorkerPair`]) without spawning the worker.
    pub(crate) async fn new_pair(
        router_handle: UdsRouterHandle,
        stream: Option<UnixStream>,
        peer: SocketAddr,
        pathnames: Vec<String>,
    ) -> Result<(Self, WorkerPair)> {
        let tx_addr = Address::random_tagged("UdsSendWorker_tx_addr");
        let int_addr = Address::random_tagged("UdsSendWorker_int_addr");
        let sender = UdsSendWorker::new(router_handle, stream, peer.clone(), int_addr);
        Ok((
            sender,
            WorkerPair {
                paths: pathnames,
                peer,
                tx_addr,
            },
        ))
    }

    /// Create a ([`UdsSendWorker`],[`WorkerPair`]) while spawning and starting the worker.
    pub(crate) async fn start_pair(
        ctx: &Context,
        router_handle: UdsRouterHandle,
        stream: Option<UnixStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        trace!("Creating new UDS worker pair");
        let (mut worker, pair) = Self::new_pair(router_handle, stream, peer, hostnames).await?;

        // TODO: @uds (Oakley) determine why his is considered bad in equivalent usage found here:
        // https://github.com/build-trust/ockam/blob/5e5a9ddc557daa2e5183d83fb95a821062c2efcf/implementations/rust/ockam/ockam_transport_tcp/src/workers/sender.rs#L134
        let rx_addr = Address::random_tagged("UdsRecvProcessor");
        worker.rx_addr = Some(rx_addr.clone());

        // TODO: @ac 0#UdsSendWorker_tx_addr
        // in:  0#UdsSendWorker_tx_addr_9  <=  [0#UdsRouter_main_addr_0]
        // out: n/a
        let tx_mailbox = Mailbox::new(
            pair.tx_addr(),
            Arc::new(AllowAll),
            // Arc::new(ockam_core::AllowSourceAddress(udsrouter_main_addr)),
            Arc::new(AllowAll),
            // Arc::new(ockam_core::DenyAll),
        );

        // @ac 0#UdsSendWorker_int_addr
        // in:  0#UdsSendWorker_int_addr_10  <=  [0#UdsRecvProcessor_12]
        // out: n/a
        let internal_mailbox = Mailbox::new(
            worker.internal_addr().clone(),
            Arc::new(AllowAll),
            // Arc::new(ockam_core::AllowSourceAddress(rx_addr)),
            Arc::new(AllowAll),
            // Arc::new(ockam_core::DenyAll),
        );

        WorkerBuilder::with_mailboxes(Mailboxes::new(tx_mailbox, vec![internal_mailbox]), worker)
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
impl Worker for UdsSendWorker {
    type Context = Context;
    type Message = Any;

    /// Connect to the UDS socket.
    ///
    /// Spawn a UDS Recceiver worker to processes incoming UDS messages
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        let path = match self.peer.as_pathname() {
            Some(p) => p,
            None => {
                debug!("Failed to determine peer path.");
                self.stop_and_unregister(ctx).await?;

                return Err(TransportError::InvalidAddress.into());
            }
        };

        if self.tx.is_none() {
            let path_display = path.display();
            debug!(addr = %path_display, "Connecting");

            let connection = match UnixStream::connect(path).await {
                Ok(c) => {
                    debug!(addr = %path_display, "Connected");
                    c
                }
                Err(e) => {
                    debug!(addr = %path_display, err = %e, "Failed to connect");
                    self.stop_and_unregister(ctx).await?;

                    return Err(TransportError::from(e).into());
                }
            };

            let sock = SockRef::from(&connection);

            // TODO: @uds enable an automatic keep alive system
            // This only enabled the socket to allow keep alive packets
            // It does not seem yet like socket2 supports an automatic interval
            // keep alive
            if let Err(e) = sock.set_keepalive(true) {
                error!("Failed to set so_keepalive to true: {}", e);
            }

            let (rx, tx) = connection.into_split();
            self.rx = Some(rx);
            self.tx = Some(tx);
        }

        let rx = self.rx.take().ok_or(TransportError::GenericIo)?;

        let rx_addr = if let Some(rx_addr) = &self.rx_addr {
            rx_addr.clone()
        } else {
            // TODO: @uds (Oakley) determine why his is considered bad in equivalent usage found here:
            // https://github.com/build-trust/ockam/blob/5e5a9ddc557daa2e5183d83fb95a821062c2efcf/implementations/rust/ockam/ockam_transport_tcp/src/workers/sender.rs#L134
            Address::random_tagged("UdsRecvProcessor")
        };

        let receiver = UdsRecvProcessor::new(
            rx,
            format!("{}#{}", crate::UDS, path.display()).into(),
            self.internal_addr.clone(),
        );

        // TODO @ac 0#UdsRecvProcessor
        // in:  n/a
        // out: 0#UdsRecvProcessor_12  =>  [0#UdsPortalWorker_remote_6, 0#UdsSendWorker_int_addr_10, 0#outlet]
        let mailbox = Mailbox::new(rx_addr, Arc::new(AllowAll), Arc::new(AllowAll));
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), receiver)
            .start(ctx)
            .await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if let Some(rx_addr) = self.rx_addr.take() {
            let _ = ctx.stop_processor(rx_addr).await;
        }

        Ok(())
    }

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
            let msg = UdsSendWorkerMsg::decode(msg.payload())?;

            match msg {
                UdsSendWorkerMsg::ConnectionClosed => {
                    warn!("Stopping sender due to closed connection");
                    // No need to stop Receiver as it notified us about connection drop and will
                    // stop itself
                    self.rx_addr = None;
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
                warn!("Failed to send message to peer");
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
