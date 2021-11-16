use crate::TransportError;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{
    async_trait, Address, Decodable, LocalMessage, Processor, Result, TransportMessage,
};
use ockam_node::Context;
use tracing::{error, info, trace};

use crate::tcp::traits::io::{AsyncRead, AsyncReadExt};

/// A TCP receiving message worker
///
/// This type will be created by [TcpSendWorker::start_pair](super::sender::TcpSendWorker::start_pair)
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for incoming TCP packets, to relay into
/// the node message system.
pub(crate) struct TcpRecvProcessor<R> {
    peer_addr: Address,
    tcp_stream: R,
    cluster_name: &'static str,
}

impl<R> TcpRecvProcessor<R> {
    pub fn new(tcp_stream: R, peer_addr: Address, cluster_name: &'static str) -> Self {
        Self {
            peer_addr,
            tcp_stream,
            cluster_name,
        }
    }
}

#[async_trait]
impl<R> Processor for TcpRecvProcessor<R>
where
    R: AsyncRead + Send + Unpin + 'static,
{
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(self.cluster_name).await
    }

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        let mut len_buf = [0; 2]; // TODO Optimization: read into unitialized buffer
        let len = match self.tcp_stream.read_exact(&mut len_buf[..]).await {
            Ok(_) => u16::from_be_bytes(len_buf),
            Err(e) => {
                trace!("Got error: {:?} while processing new tcp packets", e);
                info!(
                    "Connection to peer '{}' was closed; dropping stream",
                    self.peer_addr
                );
                return Ok(false);
            }
        };

        trace!("Received message header for {} bytes", len);

        // Allocate a buffer of that size
        // TODO allocation(Very similar case as bufer allocation for sockets[in net/stack.rs])
        let mut buf = vec![0; len as usize];

        // Then Read into the buffer
        match self.tcp_stream.read_exact(&mut buf).await {
            Ok(_) => {}
            _ => {
                error!("Failed to receive message of length: {}", len);
                return Ok(true);
            }
        }

        // Deserialize the message now
        let mut msg = TransportMessage::decode(&buf).map_err(|_| TransportError::RecvBadMessage)?;

        // Heartbeat message
        if msg.onward_route.next().is_err() {
            trace!("Got heartbeat message from: {}", self.peer_addr);
        }

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
