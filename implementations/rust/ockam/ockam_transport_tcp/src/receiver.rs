use ockam::{async_worker, Context, Result, TransportMessage, Worker};
use tokio::net::tcp::OwnedReadHalf;

pub struct TcpRecvWorker {
    pub(crate) rx: OwnedReadHalf,
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

        // TODO: read from TCP stream here

        ctx.stop_worker(self_addr).await?;
        Ok(())
    }
}
