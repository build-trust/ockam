use crate::{
    parse_socket_addr, TcpInletListenProcessor, TcpListenProcessor, TcpPortalWorker,
    TcpRouterRequest, TcpRouterResponse, WorkerPair, TCP,
};
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, AsyncTryClone, Result, Route};
use ockam_node::Context;
use ockam_transport_core::TransportError;

/// A handle to connect to a TcpRouter
///
/// Dropping this handle is harmless.
pub(crate) struct TcpRouterHandle {
    ctx: Context,
    api_addr: Address,
}

#[async_trait]
impl AsyncTryClone for TcpRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self.ctx.new_context(Address::random_local()).await?;
        Ok(Self::new(child_ctx, self.api_addr.clone()))
    }
}

impl TcpRouterHandle {
    /// Create a new `TcpRouterHandle` with the given address
    pub(crate) fn new(ctx: Context, api_addr: Address) -> Self {
        TcpRouterHandle { ctx, api_addr }
    }

    /// Return a reference to the router handle's [`Context`]
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

impl TcpRouterHandle {
    /// Bind an incoming connection listener for this router
    pub async fn bind(&self, addr: impl Into<SocketAddr>) -> Result<()> {
        let socket_addr = addr.into();
        TcpListenProcessor::start(&self.ctx, self.async_try_clone().await?, socket_addr).await
    }

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<Address> {
        let mut child_ctx = self.ctx.new_context(Address::random_local()).await?;

        child_ctx
            .send(
                self.api_addr.clone(),
                TcpRouterRequest::Connect {
                    peer: peer.as_ref().to_string(),
                },
            )
            .await?;

        let response = child_ctx
            .receive::<TcpRouterResponse>()
            .await?
            .take()
            .body();

        if let TcpRouterResponse::Connect(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Disconnect an outgoing TCP connection on an existing transport
    pub async fn disconnect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        let mut child_ctx = self.ctx.new_context(Address::random_local()).await?;

        child_ctx
            .send(
                self.api_addr.clone(),
                TcpRouterRequest::Disconnect {
                    peer: peer.as_ref().to_string(),
                },
            )
            .await?;

        let response = child_ctx
            .receive::<TcpRouterResponse>()
            .await?
            .take()
            .body();

        if let TcpRouterResponse::Disconnect(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let tcp_address = Address::new(TCP, pair.peer().to_string());
        let mut accepts = vec![tcp_address];
        accepts.extend(pair.hostnames().iter().map(|x| Address::new(TCP, x)));
        let self_addr = pair.tx_addr();

        let mut child_ctx = self.ctx.new_context(Address::random_local()).await?;
        child_ctx
            .send(
                self.api_addr.clone(),
                TcpRouterRequest::Register { accepts, self_addr },
            )
            .await?;

        let response = child_ctx
            .receive::<TcpRouterResponse>()
            .await?
            .take()
            .body();

        if let TcpRouterResponse::Register(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Unregister the conenction worker for the given `Address`
    pub async fn unregister(&self, self_addr: Address) -> Result<()> {
        let mut child_ctx = self.ctx.new_context(Address::random_local()).await?;

        child_ctx
            .send(
                self.api_addr.clone(),
                TcpRouterRequest::Unregister { self_addr },
            )
            .await?;

        let response = child_ctx
            .receive::<TcpRouterResponse>()
            .await?
            .take()
            .body();

        if let TcpRouterResponse::Unregister(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
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
        }
        // Nothing worked, return an error
        else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok((peer_addr, hostnames))
    }
}

impl TcpRouterHandle {
    /// Bind an incoming portal inlet connection listener for this router
    pub async fn bind_inlet(
        &self,
        outlet_listener_route: impl Into<Route>,
        addr: impl Into<SocketAddr>,
    ) -> Result<Address> {
        let socket_addr = addr.into();
        let addr =
            TcpInletListenProcessor::start(&self.ctx, outlet_listener_route.into(), socket_addr)
                .await?;

        Ok(addr)
    }

    /// Establish an outgoing TCP connection for Portal Outlet
    pub async fn connect_outlet(
        &self,
        peer: impl Into<String>,
        pong_route: Route,
    ) -> Result<Address> {
        let (peer_addr, _) = Self::resolve_peer(peer)?;

        let address = TcpPortalWorker::new_outlet(&self.ctx, peer_addr, pong_route).await?;

        Ok(address)
    }

    /// Stop the inlet's [`TcpInletListenProcessor`]
    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_processor(addr).await?;
        Ok(())
    }

    /// Stop the inlet's [`TcpPortalWorker`]
    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}
