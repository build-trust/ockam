use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::protocol::Message as WebSocketMessage;

use crate::error::WebSocketError;
use ockam_core::{
    async_trait, route, Address, AllowAll, Any, Decodable, Encodable, LocalMessage, Mailbox,
    Mailboxes, Result, Routed, TransportMessage, Worker,
};
use ockam_node::{Context, DelayedEvent, WorkerBuilder};
use ockam_transport_core::TransportError;

use crate::workers::{
    AsyncStream, TcpClientStream, TcpServerStream, WebSocketRecvProcessor, WebSocketStream,
};
use crate::WebSocketAddress;

/// Transmit and receive peers of a WebSocket connection.
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
    ///
    /// The WebSocket stream is created when the `WebSocketSendWorker` is initialized.
    pub(crate) async fn from_client(
        ctx: &Context,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        trace!("Creating new WS worker pair");

        let internal_addr = Address::random_tagged("WebSocketSender.internal.from_client");
        let sender = WebSocketSendWorker::<TcpClientStream>::new(
            peer,
            internal_addr.clone(),
            DelayedEvent::create(ctx, internal_addr.clone(), vec![]).await?,
        );

        let tx_addr = Address::random_tagged("WebSocketSender.tx_addr.from_client");

        let mailboxes = Mailboxes::new(
            Mailbox::new(
                tx_addr.clone(),
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            ),
            vec![Mailbox::new(
                internal_addr,
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            )],
        );
        WorkerBuilder::with_mailboxes(mailboxes, sender)
            .start(ctx)
            .await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            peer: WebSocketAddress::from(peer).into(),
            tx_addr,
        })
    }

    /// Spawn instances of `WebSocketSendWorker` and `WebSocketRecvProcessor` and
    /// returns a `WorkerPair` instance that will be registered by the `WebSocketRouter`.
    pub(crate) async fn from_server(
        ctx: &Context,
        stream: WebSocketStream<TcpServerStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        trace!("Creating new WS worker pair");

        let internal_addr = Address::random_tagged("WebSocketSender.internal.from_server");
        let sender = WebSocketSendWorker::<TcpServerStream>::new(
            stream,
            peer,
            internal_addr.clone(),
            DelayedEvent::create(ctx, internal_addr.clone(), vec![]).await?,
        );

        let tx_addr = Address::random_tagged("WebSocketSender.tx_addr.from_server");
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                tx_addr.clone(),
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            ),
            vec![Mailbox::new(
                internal_addr,
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            )],
        );
        WorkerBuilder::with_mailboxes(mailboxes, sender)
            .start(ctx)
            .await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            peer: WebSocketAddress::from(peer).into(),
            tx_addr,
        })
    }
}

/// A WebSocket sending message worker.
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct WebSocketSendWorker<S>
where
    S: AsyncStream,
{
    ws_stream: Option<SplitStream<WebSocketStream<S>>>,
    ws_sink: Option<SplitSink<WebSocketStream<S>, WebSocketMessage>>,
    peer: SocketAddr,
    internal_addr: Address,
    heartbeat: DelayedEvent<Vec<u8>>,
    heartbeat_interval: Option<Duration>,
}

impl<S> WebSocketSendWorker<S>
where
    S: AsyncStream,
{
    async fn handle_initialize(&mut self, ctx: &mut Context) -> Result<()> {
        if let Some(ws_stream) = self.ws_stream.take() {
            let rx_addr = Address::random_tagged("WebSocketSendWorker.rx_addr");
            let receiver = WebSocketRecvProcessor::new(ws_stream, self.peer);
            ctx.start_processor_with_access_control(
                rx_addr.clone(),
                receiver,
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            )
            .await?;
        } else {
            return Err(TransportError::GenericIo.into());
        }

        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        self.schedule_heartbeat().await?;
        Ok(())
    }

    async fn schedule_heartbeat(&mut self) -> Result<()> {
        let heartbeat_interval = match &self.heartbeat_interval {
            Some(hi) => *hi,
            None => return Ok(()),
        };

        self.heartbeat.schedule(heartbeat_interval).await
    }

    /// Receive messages from the `WebSocketRouter` to send
    /// across the `WebSocketStream` to the next remote peer.
    async fn handle_msg(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        self.heartbeat.cancel();

        let ws_sink = if let Some(ws_sink) = &mut self.ws_sink {
            ws_sink
        } else {
            return Err(TransportError::PeerNotFound.into());
        };

        let recipient = msg.msg_addr();
        if recipient == self.internal_addr {
            let msg = TransportMessage::v1(route![], route![], vec![]);
            // Sending empty heartbeat
            if ws_sink
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
            if ws_sink.send(msg).await.is_err() {
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

impl WebSocketSendWorker<TcpServerStream> {
    fn new(
        stream: WebSocketStream<TcpServerStream>,
        peer: SocketAddr,
        internal_addr: Address,
        heartbeat: DelayedEvent<Vec<u8>>,
    ) -> Self {
        let (ws_sink, ws_stream) = stream.split();
        Self {
            ws_sink: Some(ws_sink),
            ws_stream: Some(ws_stream),
            peer,
            internal_addr,
            heartbeat,
            heartbeat_interval: None,
        }
    }
}

impl WebSocketSendWorker<TcpClientStream> {
    fn new(peer: SocketAddr, internal_addr: Address, heartbeat: DelayedEvent<Vec<u8>>) -> Self {
        Self {
            ws_stream: None,
            ws_sink: None,
            peer,
            internal_addr,
            heartbeat,
            heartbeat_interval: None,
        }
    }

    async fn initialize_stream(&mut self) -> Result<()> {
        if self.ws_stream.is_none() {
            let peer = WebSocketAddress::from(self.peer).to_string();
            let (stream, _) = tokio_tungstenite::connect_async(peer)
                .await
                .map_err(WebSocketError::from)?;
            let (ws_sink, ws_stream) = stream.split();
            self.ws_sink = Some(ws_sink);
            self.ws_stream = Some(ws_stream);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Worker for WebSocketSendWorker<TcpServerStream> {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.handle_initialize(ctx).await?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.handle_msg(ctx, msg).await
    }
}

#[async_trait::async_trait]
impl Worker for WebSocketSendWorker<TcpClientStream> {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.initialize_stream().await?;
        self.handle_initialize(ctx).await?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.handle_msg(ctx, msg).await
    }
}
