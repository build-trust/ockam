use std::net::{SocketAddr, ToSocketAddrs};

use ockam_core::{async_trait, Address, AsyncTryClone, Result, RouterMessage};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::workers::{WebSocketListenProcessor, WorkerPair};
use crate::{parse_socket_addr, WebSocketAddr, WebSocketError, WS};

/// A handle to connect to a WebSocketRouter
///
/// Dropping this handle is harmless.
pub(crate) struct WebSocketRouterHandle {
    ctx: Context,
    addr: Address,
}

#[async_trait]
impl AsyncTryClone for WebSocketRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self.ctx.new_context(Address::random(0)).await?;
        Ok(Self::new(child_ctx, self.addr.clone()))
    }
}

impl WebSocketRouterHandle {
    pub(crate) fn new(ctx: Context, addr: Address) -> Self {
        Self { ctx, addr }
    }

    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let ws_address: Address = format!("{}#{}", WS, pair.peer()).into();
        let mut accepts = vec![ws_address];
        accepts.extend(
            pair.hostnames()
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", WS, x))),
        );
        let self_addr = pair.tx_addr();
        self.ctx
            .send(
                self.addr.clone(),
                RouterMessage::Register { accepts, self_addr },
            )
            .await
    }

    /// Bind an incoming connection listener for this router
    pub async fn bind(&self, addr: impl Into<SocketAddr>) -> Result<()> {
        let socket_addr = addr.into();
        WebSocketListenProcessor::start(&self.ctx, self.async_try_clone().await?, socket_addr).await
    }

    pub(crate) fn resolve_peer(peer: impl Into<String>) -> Result<(SocketAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let hostnames;

        // Try to parse as SocketAddr
        if let Ok(p) = parse_socket_addr(peer_str.clone()) {
            peer_addr = p;
            hostnames = vec![];
        }
        // Try to resolve hostname
        else if let Ok(mut iter) = peer_str.to_socket_addrs() {
            // FIXME: We only take ipv4 for now
            if let Some(p) = iter.find(|x| x.is_ipv4()) {
                peer_addr = p;
            } else {
                return Err(TransportError::InvalidAddress.into());
            }

            hostnames = vec![peer_str];
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok((peer_addr, hostnames))
    }

    /// Establish an outgoing WS connection on an existing transport
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        let (peer_addr, hostnames) = Self::resolve_peer(peer.as_ref())?;
        let ws_peer_addr = WebSocketAddr::from(peer_addr);
        let (stream, _) = tokio_tungstenite::connect_async(ws_peer_addr.to_string())
            .await
            .map_err(WebSocketError::from)?;
        let pair = WorkerPair::new(&self.ctx, stream, peer_addr, hostnames).await?;
        self.register(&pair).await
    }
}
