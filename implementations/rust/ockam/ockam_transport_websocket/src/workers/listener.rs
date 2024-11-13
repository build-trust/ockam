use std::net::SocketAddr;

use tokio::net::TcpListener;

use ockam_core::{async_trait, Address, AllowAll, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::{error::WebSocketError, workers::WorkerPair, WebSocketRouterHandle};

/// A worker that runs in the background as a `Processor` waiting for incoming
/// clients' connections.
///
/// When a new connection is established, a new `WorkerPair` is spawned and
/// registered by the router.
pub(crate) struct WebSocketListenProcessor {
    inner: TcpListener,
    router_handle: WebSocketRouterHandle,
}

impl WebSocketListenProcessor {
    /// Create and start a new instance bound to the given `addr`.
    pub(crate) async fn start(
        ctx: &Context,
        router_handle: WebSocketRouterHandle,
        addr: SocketAddr,
    ) -> Result<SocketAddr> {
        debug!("Binding WebSocketListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let saddr = inner.local_addr().map_err(TransportError::from)?;
        let processor = Self {
            inner,
            router_handle,
        };
        let waddr = Address::random_tagged("WebSocketListenProcessor");
        ctx.start_processor_with_access_control(
            waddr, processor, AllowAll, // FIXME: @ac
            AllowAll, // FIXME: @ac
        )?;
        Ok(saddr)
    }
}

#[async_trait]
impl Processor for WebSocketListenProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        debug!("Waiting for incoming TCP connection...");

        // Wait for an incoming connection
        let (tcp_stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;
        let ws_stream = tokio_tungstenite::accept_async(tcp_stream)
            .await
            .map_err(WebSocketError::from)?;
        debug!("TCP connection accepted");

        // Spawn a connection worker for it
        let pair = WorkerPair::from_server(ctx, ws_stream, peer, vec![])?;

        // Register the connection with the local TcpRouter
        self.router_handle.register(&pair).await?;
        debug!("TCP connection registered");

        Ok(true)
    }
}
