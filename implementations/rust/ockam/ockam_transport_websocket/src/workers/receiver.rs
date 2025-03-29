use std::net::SocketAddr;

use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use tokio_tungstenite::WebSocketStream;

use crate::WebSocketAddress;
use ockam_core::{
    async_trait, Address, Decodable, LocalMessage, Processor, Result, TransportMessage,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::workers::AsyncStream;

/// A WebSocket receiving message worker.
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages received to the WebSocket stream
/// from the remote peer.
pub(crate) struct WebSocketRecvProcessor<S>
where
    S: AsyncStream,
{
    ws_stream: SplitStream<WebSocketStream<S>>,
    peer_addr: Address,
}

impl<S> WebSocketRecvProcessor<S>
where
    S: AsyncStream,
{
    pub(crate) fn new(ws_stream: SplitStream<WebSocketStream<S>>, peer: SocketAddr) -> Self {
        Self {
            ws_stream,
            peer_addr: WebSocketAddress::from(peer).into(),
        }
    }
}

#[async_trait]
impl<S> Processor for WebSocketRecvProcessor<S>
where
    S: AsyncStream,
{
    type Context = Context;

    /// Get next message from the WebSocket stream if there is
    /// any available, and forward it to the next hop in the route.
    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        // Get next message from the stream or abort if the stream is
        // either closed or exhausted.
        let ws_msg = match self.ws_stream.next().await {
            Some(res) => match res {
                Ok(ws_msg) => ws_msg,
                Err(_e) => {
                    debug!(
                        "Connection to peer '{}' was closed; dropping stream",
                        self.peer_addr
                    );
                    return Ok(false);
                }
            },
            None => {
                info!(
                    "Stream connected to peer '{}' is exhausted; dropping stream",
                    self.peer_addr
                );
                return Ok(false);
            }
        };

        // Extract message payload
        let encoded_msg = ws_msg.into_data();

        // Deserialize the message
        let msg =
            TransportMessage::decode(&encoded_msg).map_err(|_| TransportError::RecvBadMessage)?;
        let mut msg = LocalMessage::from_transport_message(msg);

        // Heartbeat message
        if !msg.has_next_on_onward_route() {
            trace!("Got heartbeat message from: {}", self.peer_addr);
        }

        // Insert the peer address into the return route so that
        // reply routing can be properly resolved
        msg = msg.push_front_return_route(self.peer_addr.clone());

        // Some verbose logging we may want to remove
        trace!("Message onward route: {}", msg.onward_route());
        trace!("Message return route: {}", msg.return_route());

        // Forward the message to the next hop in the route
        ctx.forward(msg).await?;

        Ok(true)
    }
}
