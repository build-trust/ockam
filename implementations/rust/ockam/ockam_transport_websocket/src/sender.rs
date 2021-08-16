use std::net::SocketAddr;

use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use ockam_core::{async_trait, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::protocol::Message as WebSocketMessage;
use tokio_tungstenite::WebSocketStream;

/// A WebSocket sending message worker
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub struct WebSocketSendWorker<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) ws_sink: SplitSink<WebSocketStream<AsyncStream>, WebSocketMessage>,
    pub(crate) peer: SocketAddr,
}

#[async_trait::async_trait]
impl<AsyncStream> Worker for WebSocketSendWorker<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Message = TransportMessage;
    type Context = Context;

    // WebSocketSendWorker will receive messages from the WebSocketRouter to send
    // across the TcpStream to the next remote peer.
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        mut msg: Routed<TransportMessage>,
    ) -> Result<()> {
        trace!("Handling message in WebSocketSendWorker");

        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        msg.onward_route.step()?;

        // Create a message buffer with pre-pended length
        let msg = serde_bare::to_vec(&msg.body()).map_err(|_| TransportError::SendBadMessage)?;
        if self
            .ws_sink
            .send(WebSocketMessage::from(msg))
            .await
            .is_err()
        {
            warn!("Failed to send message to peer {}", self.peer);
            ctx.stop_worker(ctx.address()).await?;
        }

        Ok(())
    }
}
