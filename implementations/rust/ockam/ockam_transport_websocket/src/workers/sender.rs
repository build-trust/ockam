use std::net::SocketAddr;
use std::time::Duration;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{tungstenite::protocol::Message as WebSocketMessage, WebSocketStream};

use ockam_core::{
    async_trait, route, Address, Any, Decodable, Encodable, LocalMessage, Result, Routed,
    TransportMessage, Worker,
};
use ockam_node::{Context, DelayedEvent};

use crate::workers::{AsyncStream, WebSocketRecvProcessor};
use crate::WebSocketAddress;

/// Transmit and receive peers of a WebSocket connection
#[derive(Debug)]
pub(crate) struct WorkerPair {
    hostnames: Vec<String>,
    peer: Address,
    tx_addr: Address,
}

impl WorkerPair {
    pub(crate) fn hostnames(&self) -> &[String] {
        &self.hostnames
    }
    pub(crate) fn peer(&self) -> Address {
        self.peer.clone()
    }
    pub(crate) fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }

    /// Spawn instances of `WebSocketSendWorker` and `WebSocketRecvProcessor` and
    /// returns a `WorkerPair` instance that will be registered by the `WebSocketRouter`.
    pub(crate) async fn new<S>(
        ctx: &Context,
        stream: WebSocketStream<S>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair>
    where
        S: AsyncStream,
    {
        trace!("Creating new WS worker pair");

        let internal_addr = Address::random_local();
        let (ws_sink, ws_stream) = stream.split();
        let sender = WebSocketSendWorker::new(
            ws_sink,
            peer,
            internal_addr.clone(),
            DelayedEvent::create(ctx, internal_addr.clone(), vec![]).await?,
        );

        let tx_addr = Address::random_local();
        ctx.start_worker(vec![tx_addr.clone(), internal_addr], sender)
            .await?;

        let rx_addr = Address::random_local();
        let receiver = WebSocketRecvProcessor::new(ws_stream, peer);
        ctx.start_processor(rx_addr.clone(), receiver).await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            peer: WebSocketAddress::from(peer).into(),
            tx_addr,
        })
    }
}

/// A WebSocket sending message worker
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct WebSocketSendWorker<S>
where
    S: AsyncStream,
{
    ws_sink: SplitSink<WebSocketStream<S>, WebSocketMessage>,
    peer: SocketAddr,
    internal_addr: Address,
    heartbeat: DelayedEvent<Vec<u8>>,
    heartbeat_interval: Option<Duration>,
}

impl<S> WebSocketSendWorker<S>
where
    S: AsyncStream,
{
    fn new(
        ws_sink: SplitSink<WebSocketStream<S>, WebSocketMessage>,
        peer: SocketAddr,
        internal_addr: Address,
        heartbeat: DelayedEvent<Vec<u8>>,
    ) -> Self {
        Self {
            ws_sink,
            peer,
            internal_addr,
            heartbeat,
            heartbeat_interval: None,
        }
    }

    async fn schedule_heartbeat(&mut self) -> Result<()> {
        let heartbeat_interval = match &self.heartbeat_interval {
            Some(hi) => *hi,
            None => return Ok(()),
        };

        self.heartbeat.schedule(heartbeat_interval).await
    }
}

#[async_trait::async_trait]
impl<S> Worker for WebSocketSendWorker<S>
where
    S: AsyncStream,
{
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        self.schedule_heartbeat().await?;
        Ok(())
    }

    /// It will receive messages from the `WebSocketRouter` to send
    /// across the `WebSocketStream` to the next remote peer.
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.heartbeat.cancel();

        let recipient = msg.msg_addr();
        if recipient == self.internal_addr {
            let msg = TransportMessage::v1(route![], route![], vec![]);
            // Sending empty heartbeat
            if self
                .ws_sink
                .send(WebSocketMessage::from(msg.encode()?))
                .await
                .is_err()
            {
                warn!("Failed to send heartbeat to peer {}", self.peer);
                ctx.stop_worker(ctx.address()).await?;

                return Ok(());
            }
            debug!("Sent heartbeat to peer {}", self.peer);
        } else {
            let mut msg = LocalMessage::decode(msg.payload())?.into_transport_message();

            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            msg.onward_route.step()?;

            let msg = WebSocketMessage::from(msg.encode()?);
            if self.ws_sink.send(msg).await.is_err() {
                warn!("Failed to send message to peer {}", self.peer);
                ctx.stop_worker(ctx.address()).await?;
                return Ok(());
            }
            debug!("Sent message to peer {}", self.peer);
        }

        self.schedule_heartbeat().await?;

        Ok(())
    }
}
