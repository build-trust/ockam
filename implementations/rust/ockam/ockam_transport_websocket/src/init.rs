use std::net::SocketAddr;

use futures_util::StreamExt;
use ockam_core::{Address, Result};
use ockam_node::Context;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;

use crate::atomic::{self, ArcBool};
use crate::receiver::WebSocketRecvWorker;
use crate::{WebSocketAddr, WebSocketError, WebSocketRouterHandle, WebSocketSendWorker};

/// Transmit and receive peers of a WebSocket connection
#[derive(Debug)]
pub struct WorkerPair {
    pub peer: SocketAddr,
    pub tx_addr: Address,
    pub rx_addr: Address,
    run: ArcBool,
}

impl WorkerPair {
    /// Stop the worker pair
    pub async fn stop(self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.tx_addr).await?;
        ctx.stop_worker(self.rx_addr).await?;
        atomic::stop(&self.run);
        Ok(())
    }

    fn from_peer(peer: SocketAddr) -> Self {
        Self {
            peer,
            tx_addr: Address::random(0),
            rx_addr: Address::random(0),
            run: atomic::new(true),
        }
    }
}

impl WorkerPair {
    pub(crate) async fn with_stream<AsyncStream>(
        ctx: &Context,
        stream: WebSocketStream<AsyncStream>,
        peer: SocketAddr,
    ) -> Result<Self>
    where
        AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        trace!("Creating new worker pair from stream");
        let (ws_sink, ws_stream) = stream.split();
        let WorkerPair {
            peer,
            rx_addr,
            tx_addr,
            run,
        } = WorkerPair::from_peer(peer);
        let sender = WebSocketSendWorker { ws_sink, peer };
        let receiver = WebSocketRecvWorker {
            ws_stream,
            run: run.clone(),
            peer_addr: format!("{}#{}", crate::WS, peer).into(),
        };

        // Derive local worker addresses, and start them
        ctx.start_worker(tx_addr.clone(), sender).await?;
        ctx.start_worker(rx_addr.clone(), receiver).await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            peer,
            tx_addr,
            rx_addr,
            run,
        })
    }

    async fn start(ctx: &Context, peer: WebSocketAddr) -> Result<Self> {
        debug!("Starting worker connection to remote {}", &peer);
        let (stream, _) = tokio_tungstenite::connect_async(peer.to_string())
            .await
            .map_err(WebSocketError::from)?;
        Self::with_stream(ctx, stream, peer.into()).await
    }
}

/// Start a new pair of WebSocket connection workers
///
/// One worker handles outgoing messages, while another handles
/// incoming messages. The local worker address is chosen based on
/// the peer the worker is meant to be connected to.
pub async fn start_connection(
    ctx: &Context,
    router: &WebSocketRouterHandle,
    peer: WebSocketAddr,
) -> Result<()> {
    let pair = WorkerPair::start(ctx, peer).await?;
    router.register(&pair).await?;
    Ok(())
}
