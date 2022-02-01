use crate::{
    parse_socket_addr, TcpInletListenProcessor, TcpListenProcessor, TcpPortalWorker, TcpSendWorker,
    WorkerPair, TCP,
};
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, AsyncTryClone, Result, Route, RouterMessage};
use ockam_node::Context;
use ockam_transport_core::TransportError;

/// A handle to connect to a TcpRouter
///
/// Dropping this handle is harmless.
pub(crate) struct TcpRouterHandle {
    ctx: Context,
    addr: Address,
}

impl TcpRouterHandle {
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

#[async_trait]
impl AsyncTryClone for TcpRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self.ctx.new_context(Address::random(0)).await?;
        Ok(Self::new(child_ctx, self.addr.clone()))
    }
}

impl TcpRouterHandle {
    pub(crate) fn new(ctx: Context, addr: Address) -> Self {
        TcpRouterHandle { ctx, addr }
    }
}

impl TcpRouterHandle {
    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let tcp_address: Address = format!("{}#{}", TCP, pair.peer()).into();
        let mut accepts = vec![tcp_address];
        accepts.extend(
            pair.hostnames()
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", TCP, x))),
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
        TcpListenProcessor::start(&self.ctx, self.async_try_clone().await?, socket_addr).await
    }

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

    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_processor(addr).await?;

        Ok(())
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

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        let (peer_addr, hostnames) = Self::resolve_peer(peer.as_ref())?;

        let pair = TcpSendWorker::start_pair(&self.ctx, None, peer_addr, hostnames).await?;
        self.register(&pair).await?;

        Ok(())
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

    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}
