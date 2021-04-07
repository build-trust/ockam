use crate::{
    atomic::{self, ArcBool},
    TcpError,
};
use ockam::{async_worker, Address, Context, Result, TransportMessage, Worker};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

/// A TCP receiving message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for incoming TCP packets, to relay into
/// the node message system.
pub struct TcpRecvWorker {
    pub(crate) rx: OwnedReadHalf,
    pub(crate) run: ArcBool,
    pub(crate) peer_addr: Address,
}

#[async_worker]
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
        let self_addr = ctx.primary_address();

        // Run in a loop until TcpWorkerPair::stop() is called
        // FIXME: see ArcBool future note
        while atomic::check(&self.run) {
            // First read a message length header...
            let len = match self.rx.read_u16().await {
                Ok(len) => len,
                Err(e) => {
                    error!("Failed to receive message: {}", e);
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
            let mut msg: TransportMessage =
                serde_bare::from_slice(buf.as_slice()).map_err(|_| TcpError::RecvBadMessage)?;

            // Insert the peer address into the return route so that
            // reply routing can be properly resolved
            msg.return_.modify().prepend(self.peer_addr.clone());

            // Some verbose logging we may want to remove
            trace!("Message onward route: {}", msg.onward);
            trace!("Message return route: {}", msg.return_);

            // FIXME: if we need to re-route (i.e. send it to another
            // domain specific router) the message here, use
            // send_message, instead of forward_message.

            // Forward the message to the final destination worker,
            // which consumes the TransportMessage and yields the
            // final message type
            ctx.forward_message(msg).await?;
        }

        // Stop the worker to not fall into the next read loop
        ctx.stop_worker(self_addr).await?;
        Ok(())
    }
}
