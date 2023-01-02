use core::str::FromStr;
use std::net::{SocketAddr, ToSocketAddrs};

use ockam_core::{async_trait, Address, AsyncTryClone, DenyAll, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::router::{WebSocketRouterRequest, WebSocketRouterResponse};
use crate::workers::{WebSocketListenProcessor, WorkerPair};
use crate::{parse_socket_addr, WebSocketAddress};

/// A handle to connect to a WebSocketRouter.
///
/// Dropping this handle is harmless.
pub(crate) struct WebSocketRouterHandle {
    ctx: Context,
    api_addr: Address,
}

#[async_trait]
impl AsyncTryClone for WebSocketRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self
            .ctx
            .new_detached(
                Address::random_tagged("WebSocketRouterHandle.async_try_clone.detached"),
                DenyAll,
                DenyAll,
            )
            .await?;
        Ok(Self::new(child_ctx, self.api_addr.clone()))
    }
}

impl WebSocketRouterHandle {
    pub(crate) fn new(ctx: Context, api_addr: Address) -> Self {
        Self { ctx, api_addr }
    }

    /// Register a new connection worker with this router.
    pub(crate) async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let mut accepts = vec![pair.peer()];

        accepts.extend(
            pair.hostnames()
                .iter()
                .filter_map(|x| WebSocketAddress::from_str(x).ok())
                .map(|addr| addr.into()),
        );
        let self_addr = pair.tx_addr();
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                WebSocketRouterRequest::Register { accepts, self_addr },
            )
            .await?;

        let WebSocketRouterResponse::Register(res) = response;

        res
    }

    /// Bind an incoming connection listener for this router.
    pub(crate) async fn bind(&self, addr: impl Into<SocketAddr>) -> Result<SocketAddr> {
        let socket_addr = addr.into();
        WebSocketListenProcessor::start(&self.ctx, self.async_try_clone().await?, socket_addr).await
    }

    /// Return the peer's `SocketAddr` and `hostnames` given a plain `String` address.
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

    /// Establish an outgoing WS connection on an existing transport.
    pub(crate) async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        // Get peer address and connect to it.
        let (peer_addr, hostnames) = Self::resolve_peer(peer.as_ref())?;

        // Create a new `WorkerPair` for the given peer, initializing a new pair
        // of sender worker and receiver processor.
        let pair = WorkerPair::from_client(&self.ctx, peer_addr, hostnames).await?;

        // Handle node's register request.
        self.register(&pair).await
    }
}
