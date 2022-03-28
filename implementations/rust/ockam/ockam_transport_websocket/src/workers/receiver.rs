use std::net::SocketAddr;

use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use tokio_tungstenite::WebSocketStream;

use ockam_core::{
    async_trait, Address, Decodable, LocalMessage, Processor, Result, TransportMessage,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::workers::AsyncStream;

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
    pub fn new(ws_stream: SplitStream<WebSocketStream<S>>, peer: SocketAddr) -> Self {
        Self {
            ws_stream,
            peer_addr: format!("{}#{}", crate::WS, peer).into(),
        }
    }
}

#[async_trait::async_trait]
impl<S> Processor for WebSocketRecvProcessor<S>
where
    S: AsyncStream,
{
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        let ws_msg = match self.ws_stream.next().await {
            Some(res) => match res {
                Ok(ws_msg) => ws_msg,
                Err(_e) => {
                    info!(
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

        let encoded_msg = ws_msg.into_data();

        // Deserialize the message now
        let mut msg =
            TransportMessage::decode(&encoded_msg).map_err(|_| TransportError::RecvBadMessage)?;

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
