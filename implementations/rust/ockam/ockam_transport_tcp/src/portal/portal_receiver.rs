use crate::PortalMessage;
use async_trait::async_trait;
use ockam_core::{route, Address, Processor, Result};
use ockam_node::Context;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::info;

const BUFFER_SIZE: usize = 256;

/// A TCP receiving message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for incoming TCP packets, to relay into
/// the node message system.
pub(crate) struct TcpPortalRecvProcessor {
    rx: OwnedReadHalf,
    sender_address: Address,
}

impl TcpPortalRecvProcessor {
    pub fn new(rx: OwnedReadHalf, sender_address: Address) -> Self {
        Self { rx, sender_address }
    }
}

#[async_trait]
impl Processor for TcpPortalRecvProcessor {
    type Context = Context;

    // We are using the initialize function here to run a custom loop,
    // while never listening for messages sent to our address
    //
    // Note: when the loop exits, we _must_ call stop_worker(..) on
    // Context not to spawn a zombie task.
    //
    // Also: we must stop the TcpReceive loop when the worker gets
    // killed by the user or node.
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        let mut buf = [0u8; BUFFER_SIZE];
        let len = match self.rx.read(&mut buf).await {
            Ok(len) => len,
            Err(_e) => {
                info!("Tcp Portal connection was closed; dropping stream",);
                return Ok(false);
            }
        };

        if len != 0 {
            let mut vec = vec![0u8; len];
            vec.copy_from_slice(&buf[..len]);
            let msg = PortalMessage { binary: vec };

            // Forward the message to the next hop in the route
            ctx.send(route![self.sender_address.clone()], msg).await?;
        } else {
            info!("Tcp Portal connection is empty; dropping stream",);
            return Ok(false);
        }

        Ok(true)
    }
}
