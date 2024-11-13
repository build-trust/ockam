use std::os::unix::net::SocketAddr;

use ockam_core::{
    async_trait, compat::sync::Arc, Address, AllowAll, Any, Decodable, DenyAll, LocalMessage,
    Mailbox, Mailboxes, Message, Result, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::{encode_transport_message, TransportError};
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
    rx_addr: Address,
    rx_should_be_stopped: bool,
}

impl UdsSendWorker {
    /// Create a new [`UdsSendWorker`]
    fn new(
        router_handle: UdsRouterHandle,
        stream: Option<UnixStream>,
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

    /// Returns a reference to the [`Receiver Process Address`](ockam_core::Address)
    pub(crate) fn rx_addr(&self) -> &Address {
        &self.rx_addr
    }

    /// Create a ([`UdsSendWorker`],[`WorkerPair`]) without spawning the worker.
    pub(crate) fn new_pair(
        router_handle: UdsRouterHandle,
        stream: Option<UnixStream>,
        peer: SocketAddr,
        pathnames: Vec<String>,
    ) -> Result<(Self, WorkerPair)> {
        let role_str = if stream.is_none() {
            "initiator"
        } else {
            "responder"
        };

        let tx_addr = Address::random_tagged(&format!("UdsSendWorker_tx_addr_{role_str}"));
        let int_addr = Address::random_tagged(&format!("UdsSendWorker_int_addr_{role_str}"));
        let rx_addr = Address::random_tagged(&format!("UdsRecvProcessor_{role_str}"));
        let sender = UdsSendWorker::new(router_handle, stream, peer.clone(), int_addr, rx_addr);
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
    pub(crate) fn start_pair(
        ctx: &Context,
        router_handle: UdsRouterHandle,
        stream: Option<UnixStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        let udsrouter_main_addr = router_handle.main_addr().clone();

        trace!("Creating new UDS worker pair");
        let (worker, pair) = Self::new_pair(router_handle, stream, peer, hostnames)?;

        let tx_mailbox = Mailbox::new(
            pair.tx_addr(),
            None,
            Arc::new(ockam_core::AllowSourceAddress(udsrouter_main_addr)),
            Arc::new(ockam_core::DenyAll),
        );

        let internal_mailbox = Mailbox::new(
            worker.internal_addr().clone(),
            None,
            Arc::new(ockam_core::AllowSourceAddress(worker.rx_addr().clone())),
            Arc::new(ockam_core::DenyAll),
        );

        WorkerBuilder::new(worker)
            .with_mailboxes(Mailboxes::new(tx_mailbox, vec![internal_mailbox]))
            .start(ctx)?;

        Ok(pair)
    }

    async fn stop_and_unregister(&self, ctx: &Context) -> Result<()> {
        self.router_handle
            .unregister(ctx.primary_address().clone())
            .await?;

        ctx.stop_address(ctx.primary_address())?;

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
        let path = match self.peer.as_pathname() {
            Some(p) => p,
            None => {
                debug!("Failed to determine peer path.");
                self.stop_and_unregister(ctx).await?;

                return Err(TransportError::InvalidAddress(format!("{:?}", self.peer)))?;
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

                    return Err(TransportError::from(e))?;
                }
            };

            let sock = SockRef::from(&connection);

            // This only enabled the socket to allow keep alive packets
            // socket2 at this time (01/2023) does not support an automatic interval
            // keep alive; However as this a Unix Domain Socket, this is less
            // likely to cause issues
            if let Err(e) = sock.set_keepalive(true) {
                error!("Failed to set so_keepalive to true: {}", e);
            }

            let (rx, tx) = connection.into_split();
            self.rx = Some(rx);
            self.tx = Some(tx);
        }

        let rx = self.rx.take().ok_or(TransportError::GenericIo)?;

        let receiver = UdsRecvProcessor::new(
            rx,
            format!("{}#{}", crate::UDS, path.display()).into(),
            self.internal_addr.clone(),
        );

        ctx.start_processor_with_access_control(self.rx_addr.clone(), receiver, DenyAll, AllowAll)?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if self.rx_should_be_stopped {
            let _ = ctx.stop_address(self.rx_addr());
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
            None => return Err(TransportError::PeerNotFound)?,
        };

        let recipient = msg.msg_addr();
        if recipient == self.internal_addr {
            let msg = UdsSendWorkerMsg::decode(msg.payload())?;

            match msg {
                UdsSendWorkerMsg::ConnectionClosed => {
                    debug!("Stopping sender due to closed connection");
                    // No need to stop Receiver as it notified us about connection drop and will
                    // stop itself
                    self.rx_should_be_stopped = false;
                    self.stop_and_unregister(ctx).await?;

                    return Ok(());
                }
            }
        } else {
            let mut local_message = LocalMessage::decode(msg.payload())?;
            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            local_message = local_message.pop_front_onward_route()?;

            // Create a message buffer with prepended length
            let msg = encode_transport_message(local_message.into_transport_message())?;

            if tx.write_all(msg.as_slice()).await.is_err() {
                warn!("Failed to send message to peer");
                self.stop_and_unregister(ctx).await?;

                return Ok(());
            }
        }

        Ok(())
    }
}
