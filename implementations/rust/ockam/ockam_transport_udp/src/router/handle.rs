use std::{
    net::{SocketAddr, ToSocketAddrs},
    str::FromStr,
};

use futures_util::stream::StreamExt;
use ockam_core::{async_trait, Address, AsyncTryClone, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;

use crate::{
    parse_socket_addr,
    workers::{TransportMessageCodec, UdpListenProcessor, UdpSendWorker},
    UdpAddress,
};

use super::UdpRouterMessage;

/// A handle to connect to a UdpRouter
///
/// Dropping this handle is harmless.
pub(crate) struct UdpRouterHandle {
    ctx: Context,
    api_addr: Address,
}

#[async_trait]
impl AsyncTryClone for UdpRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self.ctx.new_detached(Address::random_local()).await?;
        Ok(Self::new(child_ctx, self.api_addr.clone()))
    }
}

impl UdpRouterHandle {
    /// Create a new `UdpRouterHandle` with given address
    pub fn new(ctx: Context, api_addr: Address) -> Self {
        Self { ctx, api_addr }
    }

    /// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
    pub fn resolve_peer(peer: impl Into<String>) -> Result<(SocketAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let hostnames;

        // Try to parse as SocketAddr
        if let Ok(p) = parse_socket_addr(peer_str.clone()) {
            peer_addr = p;
            hostnames = vec![];
        } else if let Ok(mut iter) = peer_str.to_socket_addrs() {
            // Try to resolve hostname
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

    /// Bind a listener with given address for this router
    pub async fn bind(&self, addr: impl Into<SocketAddr>) -> Result<()> {
        let socket = UdpSocket::bind(addr.into())
            .await
            .map_err(TransportError::from)?;
        let (sink, stream) = UdpFramed::new(socket, TransportMessageCodec).split();

        let tx_addr = Address::random_local();
        let sender = UdpSendWorker::new(sink);
        self.ctx.start_worker(tx_addr.clone(), sender).await?;
        UdpListenProcessor::start(&self.ctx, stream, tx_addr, self.async_try_clone().await?)
            .await?;

        Ok(())
    }

    /// Register a new worker with this router
    pub(crate) async fn register(&self, tx_addr: Address, peer: impl Into<String>) -> Result<()> {
        let (peer, hostnames) = Self::resolve_peer(peer.into())?;
        let mut accepts = vec![UdpAddress::from(peer).into()];
        accepts.extend(
            hostnames
                .iter()
                .filter_map(|s| UdpAddress::from_str(s).ok())
                .map(|addr| addr.into()),
        );

        // TODO: should we send a router request instead
        // and see if worker is already registered?
        self.ctx
            .send(
                self.api_addr.clone(),
                UdpRouterMessage::Register {
                    accepts,
                    self_addr: tx_addr,
                },
            )
            .await
    }
}
