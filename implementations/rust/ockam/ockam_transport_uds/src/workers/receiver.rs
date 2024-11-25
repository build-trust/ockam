use crate::workers::UdsSendWorkerMsg;

use ockam_core::{
    async_trait, Address, Decodable, LocalMessage, Processor, Result, TransportMessage,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::{io::AsyncReadExt, net::unix::OwnedReadHalf};
use tracing::{debug, error, trace};

/// A UDS receiving message processor
///
/// Create this processor type by calling
/// [`UdsSendWorker::start_pair`](crate::workers::UdsSendWorker)
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for UDS packets which are relayed into
/// the node messaging system.
pub(crate) struct UdsRecvProcessor {
    rx: OwnedReadHalf,
    peer_addr: Address,
    sender_internal_address: Address,
}

impl UdsRecvProcessor {
    pub fn new(rx: OwnedReadHalf, peer_addr: Address, sender_internal_address: Address) -> Self {
        Self {
            rx,
            peer_addr,
            sender_internal_address,
        }
    }
}

#[async_trait]
impl Processor for UdsRecvProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    /// Get the next message from the connection if there are any
    /// available and forward it to the next hop in the route.
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        // Run in a loop until UdsWorkerPair::stop() is called
        // First read a message length header...
        let len = match self.rx.read_u16().await {
            Ok(len) => len,
            Err(_e) => {
                debug!(
                    "Connection to peer '{}' was closed; dropping stream",
                    self.peer_addr
                );

                // Notify sender tx is closed
                ctx.send(
                    self.sender_internal_address.clone(),
                    UdsSendWorkerMsg::ConnectionClosed,
                )
                .await?;

                return Ok(false);
            }
        };

        trace!("Received message header for {} bytes", len);

        // Allocate a buffer of that size
        let mut buf = vec![0; len as usize];

        // Then read into the buffer
        match self.rx.read_exact(&mut buf).await {
            Ok(_) => {}
            _ => {
                error!("Failed to receive message of length: {}", len);
                return Ok(true);
            }
        }

        // Deserialize the message now
        let msg = TransportMessage::decode(&buf).map_err(|_| TransportError::RecvBadMessage)?;
        let mut msg = LocalMessage::from_transport_message(msg);

        // Heartbeat message
        if !msg.has_next_on_onward_route() {
            trace!("Got heartbeat message from: {}", self.peer_addr);
            return Ok(true);
        }

        // Insert the peer address into the return route so that
        // reply routing can be properly resolved
        msg = msg.push_front_return_route(&self.peer_addr);

        trace!("Message onward route: {}", msg.onward_route());
        trace!("Message return route: {}", msg.return_route());

        // Forward the message to the next hop in the route
        ctx.forward(msg).await?;

        Ok(true)
    }
}
