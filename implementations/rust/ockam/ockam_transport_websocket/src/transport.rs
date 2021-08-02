use std::net::SocketAddr;

use ockam_core::lib::str::FromStr;
use ockam_core::{async_trait, Address, Result};
use ockam_node::Context;
use tracing::debug;

use crate::addr::WebSocketAddr;
use crate::common::parse_socket_addr;
use crate::common::{Router, RouterHandler, Transport, TransportError, TransportNode};
use crate::connection_listener::ConnectionListenerWorkerWebSocket;
use crate::node::TransportNodeWebSocket;

pub struct TransportWebSocket {
    ctx: Context,
    router: RouterHandler,
}

impl TransportWebSocket {
    pub async fn new(ctx: &Context) -> Result<Self> {
        debug!("Initializing TransportWebSocket instance");
        let addr = Address::random(0);
        let ctx = ctx.new_context(addr).await?;

        let addr = Address::random(0);
        let router = Router::new(&ctx, TransportNodeWebSocket::ADDR_ID, addr).await?;

        Ok(Self { ctx, router })
    }
}

#[async_trait::async_trait]
impl Transport for TransportWebSocket {
    async fn connect(&self, peer: &str) -> Result<()> {
        debug!("Connecting [peer = {}]", peer);
        let peer = WebSocketAddr::from_str(peer)?;
        let (stream, _res) = tokio_tungstenite::connect_async(peer.to_string())
            .await
            .map_err(TransportError::from)?;
        let node = TransportNodeWebSocket::new(peer.into());
        node.start(&self.ctx, stream).await?;
        self.router.register(&node).await
    }

    async fn listen(&self, addr: &str) -> Result<()> {
        debug!("Listening [addr = {}]", addr);
        let addr = parse_socket_addr(addr)?;
        self.router
            .bind::<SocketAddr, ConnectionListenerWorkerWebSocket, TransportNodeWebSocket>(addr)
            .await
    }

    async fn shutdown(&self) -> Result<()> {
        debug!("Shutting down");
        self.router.shutdown().await
    }
}
