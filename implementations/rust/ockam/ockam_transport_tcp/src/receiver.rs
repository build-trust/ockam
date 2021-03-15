use crate::TcpError;
use ockam::{async_worker, Context, Result, TransportMessage, Worker};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
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
    pub(crate) run: Arc<AtomicBool>,
}

#[async_worker]
impl Worker for TcpRecvWorker {
    type Context = Context;
    type Message = TransportMessage;

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
        while self.run.load(Ordering::Relaxed) {
            // First read a message length header...
            let len = self.rx.read_u16().await.unwrap();

            // Allocate a buffer of that size
            let mut buf = vec![0; len as usize];

            // Then Read into the buffer
            match self.rx.read_exact(&mut buf).await {
                Ok(_) => {}
                _ => {
                    // Log failure?
                    continue;
                }
            }

            // Deserialize the message now
            let msg: TransportMessage =
                serde_bare::from_slice(buf.as_slice()).map_err(|_| TcpError::RecvBadMessage)?;

            // Figure out the next hop in the route
            let addr = msg.onward.next().unwrap();

            // Send the message
            ctx.send_message(addr.clone(), msg).await?;
        }

        // Stop the worker to not fall into the next read loop
        ctx.stop_worker(self_addr).await?;
        Ok(())
    }
}
