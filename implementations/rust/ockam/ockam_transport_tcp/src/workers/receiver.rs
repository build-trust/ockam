use crate::atomic::{self, ArcBool};
use async_trait::async_trait;
use ockam_core::{Address, LocalMessage, Result, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::{error, info, trace};

/// A TCP receiving message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for incoming TCP packets, to relay into
/// the node message system.
pub(crate) struct TcpRecvWorker {
    rx: OwnedReadHalf,
    run: ArcBool,
    peer_addr: Address,
}

impl TcpRecvWorker {
    pub fn new(rx: OwnedReadHalf, run: ArcBool, peer_addr: Address) -> Self {
        Self { rx, run, peer_addr }
    }
}

#[async_trait]
impl Worker for TcpRecvWorker {
    type Context = Context;

    // Do not actually listen for messages
    type Message = ();

    // We are using the initialize function here to run a custom loop,
    // while never listening for messages sent to our address
    //
    // Note: when the loop exits, we _must_ call stop_worker(..) on
    // Context not to spawn a zombie task.
    //
    // Also: we must stop the TcpReceive loop when the worker gets
    // killed by the user or node.
    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        let self_addr = ctx.address();

        // Run in a loop until TcpWorkerPair::stop() is called
        // FIXME: see ArcBool future note
        while atomic::check(&self.run) {
            // First read a message length header...
            let len = match self.rx.read_u16().await {
                Ok(len) => len,
                Err(_e) => {
                    info!(
                        "Connection to peer '{}' was closed; dropping stream",
                        self.peer_addr
                    );
                    break;
                }
            };

            trace!("Received message header for {} bytes", len);

            // Allocate a buffer of that size
            let mut buf = vec![0; len as usize];

            // Then Read into the buffer
            match self.rx.read_exact(&mut buf).await {
                Ok(_) => {}
                _ => {
                    error!("Failed to receive message of length: {}", len);
                    continue;
                }
            }

            // Deserialize the message now
            let mut msg: TransportMessage = serde_bare::from_slice(buf.as_slice())
                .map_err(|_| TransportError::RecvBadMessage)?;

            // Insert the peer address into the return route so that
            // reply routing can be properly resolved
            msg.return_route.modify().prepend(self.peer_addr.clone());

            // Some verbose logging we may want to remove
            trace!("Message onward route: {}", msg.onward_route);
            trace!("Message return route: {}", msg.return_route);

            // Forward the message to the next hop in the route
            ctx.forward(LocalMessage::new(msg, Vec::new())).await?;
        }

        // Stop the worker to not fall into the next read loop
        ctx.stop_worker(self_addr).await?;
        Ok(())
    }
}
