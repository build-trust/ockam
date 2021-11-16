use crate::{parse_socket_addr, TcpInletListenProcessor, TcpPortalWorker};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{async_trait, Address, AsyncTryClone, Result, Route};
use ockam_node::Context;
use ockam_transport_core::tcp::router::TcpRouterHandle as BaseHandler;
use ockam_transport_core::tcp::traits::{EndpointResolver, TokioTcpBinder, TokioTcpConnector};
use ockam_transport_core::TransportError;
use std::net::ToSocketAddrs;

pub(crate) struct TcpRouterHandle(BaseHandler<PeerResolve>);

#[async_trait]
impl AsyncTryClone for TcpRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        Ok(Self(self.0.async_try_clone().await?))
    }
}

impl From<BaseHandler<PeerResolve>> for TcpRouterHandle {
    fn from(base: BaseHandler<PeerResolve>) -> Self {
        Self(base)
    }
}

impl From<TcpRouterHandle> for BaseHandler<PeerResolve> {
    fn from(handler: TcpRouterHandle) -> Self {
        handler.0
    }
}

pub(crate) struct PeerResolve;

impl EndpointResolver for PeerResolve {
    type Hostnames = Vec<String>;

    type Peer = SocketAddr;

    fn resolve_endpoint(peer: &str) -> Result<(Self::Peer, Self::Hostnames), ockam_core::Error> {
        let peer_str: String = peer.into();
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
}

impl TcpRouterHandle {
    pub(crate) async fn connect(&self, peer: impl AsRef<str>) -> Result<()> {
        self.0.connect(peer, TokioTcpConnector).await
    }

    pub(crate) async fn bind(&self, bind_addr: SocketAddr) -> Result<()> {
        self.0.bind(bind_addr, TokioTcpBinder).await
    }

    pub(crate) fn ctx(&self) -> &Context {
        self.0.ctx()
    }
    /// Bind an incoming portal inlet connection listener for this router
    pub async fn bind_inlet(
        &self,
        outlet_listener_route: impl Into<Route>,
        addr: impl Into<SocketAddr>,
    ) -> Result<Address> {
        let socket_addr = addr.into();
        let addr =
            TcpInletListenProcessor::start(self.0.ctx(), outlet_listener_route.into(), socket_addr)
                .await?;

        Ok(addr)
    }

    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.0.ctx().stop_processor(addr).await?;

        Ok(())
    }

    /// Establish an outgoing TCP connection for Portal Outlet
    pub async fn connect_outlet(
        &self,
        peer: impl AsRef<str>,
        pong_route: Route,
    ) -> Result<Address> {
        let (peer_addr, _) = PeerResolve::resolve_endpoint(peer.as_ref())?;

        let address = TcpPortalWorker::new_outlet(self.0.ctx(), peer_addr, pong_route).await?;

        Ok(address)
    }

    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.0.ctx().stop_worker(addr).await?;
        Ok(())
    }
}
