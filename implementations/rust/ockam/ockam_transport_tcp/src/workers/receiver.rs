use ockam_core::async_trait;
use ockam_core::{Address, Decodable, LocalMessage, Processor, Result, TransportMessage};
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
pub(crate) struct TcpRecvProcessor {
    rx: OwnedReadHalf,
    peer_addr: Address,
}

impl TcpRecvProcessor {
    pub fn new(rx: OwnedReadHalf, peer_addr: Address) -> Self {
        Self { rx, peer_addr }
    }
}

#[async_trait]
impl Processor for TcpRecvProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    // We are using the initialize function here to run a custom loop,
    // while never listening for messages sent to our address
    //
    // Note: when the loop exits, we _must_ call stop_worker(..) on
    // Context not to spawn a zombie task.
    //
    // Also: we must stop the TcpReceive loop when the worker gets
    // killed by the user or node.
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        // Run in a loop until TcpWorkerPair::stop() is called
        // First read a message length header...
        let len = match self.rx.read_u16().await {
            Ok(len) => len,
            Err(_e) => {
                info!(
                    "Connection to peer '{}' was closed; dropping stream",
                    self.peer_addr
                );
                return Ok(false);
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
                return Ok(true);
            }
        }

        // Deserialize the message now
        let mut msg = TransportMessage::decode(&buf).map_err(|_| TransportError::RecvBadMessage)?;

        // Insert the peer address into the return route so that
        // reply routing can be properly resolved
        msg.return_route.modify().prepend(self.peer_addr.clone());

        // Some verbose logging we may want to remove
        trace!("Message onward route: {}", msg.onward_route);
        trace!("Message return route: {}", msg.return_route);

        // Forward the message to the next hop in the route
        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        Ok(true)
    }
}
