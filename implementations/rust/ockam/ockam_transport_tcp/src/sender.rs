use ockam::{async_worker, Context, Result, Routed, TransportMessage, Worker};
use tokio::net::tcp::OwnedWriteHalf;

pub struct TcpSendWorker {
    pub(crate) tx: OwnedWriteHalf,
}

#[async_worker]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = TransportMessage;

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(&mut self, _: &mut Context, _: Routed<TransportMessage>) -> Result<()> {
        Ok(())
    }
}
