use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ockam_core::lib::net::SocketAddr;
use ockam_core::{
    async_trait, worker, Address, LocalMessage, Result, Routed, TransportMessage, Worker,
};
use ockam_node::Context;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::protocol::Message as WebSocketMessage;
use tokio_tungstenite::WebSocketStream;
use tracing::{info, trace, warn};

use crate::common::{TransportError, TransportNode};

pub struct TransportNodeWebSocket {
    peer: SocketAddr,
    tx_addr: Address,
    rx_addr: Address,
}

impl TransportNodeWebSocket {
    pub fn new(peer: SocketAddr) -> Self {
        Self {
            peer,
            tx_addr: Address::random(0),
            rx_addr: Address::random(0),
        }
    }

    pub async fn start<AsyncStream>(
        &self,
        ctx: &Context,
        stream: WebSocketStream<AsyncStream>,
    ) -> Result<()>
    where
        AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        trace!("Starting WebSocket transport node");
        let (ws_sink, ws_stream) = stream.split();
        let tx = TransportNodeWebSocketTx {
            ws_sink,
            peer: self.peer(),
        };
        ctx.start_worker(self.tx_addr(), tx).await?;

        let rx = TransportNodeWebSocketRx {
            ws_stream,
            peer: Self::build_addr(self.peer()),
        };
        ctx.start_worker(self.rx_addr(), rx).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl TransportNode for TransportNodeWebSocket {
    const ADDR_ID: u8 = 2;

    fn peer(&self) -> SocketAddr {
        self.peer
    }

    fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }

    fn rx_addr(&self) -> Address {
        self.rx_addr.clone()
    }
}

struct TransportNodeWebSocketTx<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    ws_sink: SplitSink<WebSocketStream<AsyncStream>, WebSocketMessage>,
    peer: SocketAddr,
}

#[worker]
impl<AsyncStream> Worker for TransportNodeWebSocketTx<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    type Message = TransportMessage;
    type Context = Context;

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

struct TransportNodeWebSocketRx<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    ws_stream: SplitStream<WebSocketStream<AsyncStream>>,
    peer: Address,
}

// Quitar ws_stream y run
// Reemplazar por un inner, que es el trait WebSocketStreamProcessor

#[async_trait::async_trait]
impl<AsyncStream> Worker for TransportNodeWebSocketRx<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Do not actually listen for messages
    type Message = ();
    type Context = Context;

    // We are using the initialize function here to run a custom loop,
    // while never listening for messages sent to our address
    //
    // Note: when the loop exits, we _must_ call stop_worker(..) on
    // Context not to spawn a zombie task.
    //
    // Also: we must stop the loop when the worker gets killed by
    // the user or node.
    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        loop {
            let ws_msg = match self.ws_stream.next().await {
                Some(res) => match res {
                    Ok(ws_msg) => ws_msg,
                    Err(_e) => {
                        info!(
                            "Connection to peer '{}' was closed; dropping stream",
                            self.peer
                        );
                        break;
                    }
                },
                None => {
                    info!(
                        "Stream connected to peer '{}' is exhausted; dropping it",
                        self.peer
                    );
                    break;
                }
            };

            let data = ws_msg.into_data();
            trace!("Received message header for {} bytes", data.len());

            // Deserialize the message now
            let mut msg: TransportMessage = serde_bare::from_slice(data.as_slice())
                .map_err(|_| TransportError::RecvBadMessage)?;

            // Insert the peer address into the return route so that
            // reply routing can be properly resolved
            msg.return_route.modify().prepend(self.peer.clone());

            // Some verbose logging we may want to remove
            trace!("Message onward route: {}", msg.onward_route);
            trace!("Message return route: {}", msg.return_route);

            // FIXME: if we need to re-route (i.e. send it to another
            // domain specific router) the message here, use
            // send_message, instead of forward_message.

            // Forward the message to the final destination worker,
            // which consumes the TransportMessage and yields the
            // final message type
            ctx.forward(LocalMessage::new(msg, Vec::new())).await?;
        }

        // Stop the worker to not fall into the next read loop
        ctx.stop_worker(ctx.address()).await?;
        Ok(())
    }
}
