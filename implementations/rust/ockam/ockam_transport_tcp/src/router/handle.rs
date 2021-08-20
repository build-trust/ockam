use crate::atomic::ArcBool;
use crate::{
    parse_socket_addr, PortalWorkerPair, TcpError, TcpInletListenProcessor, TcpListenProcessor,
    WorkerPair, TCP,
};
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::{Address, Result, Route, RouterMessage};
use ockam_node::{block_future, Context};
use std::sync::Arc;

/// A handle to connect to a TcpRouter
///
/// Dropping this handle is harmless.
pub(crate) struct TcpRouterHandle {
    ctx: Context,
    addr: Address,
    run: ArcBool,
}

impl TcpRouterHandle {
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

impl Clone for TcpRouterHandle {
    fn clone(&self) -> Self {
        let child_ctx = block_future(&self.ctx.runtime(), async {
            self.ctx.new_context(Address::random(0)).await.unwrap()
        });
        Self::new(child_ctx, self.addr.clone(), self.run.clone())
    }
}

impl TcpRouterHandle {
    pub(crate) fn new(ctx: Context, addr: Address, run: ArcBool) -> Self {
        TcpRouterHandle { ctx, addr, run }
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
        TcpListenProcessor::start(&self.ctx, self.clone(), socket_addr, Arc::clone(&self.run)).await
    }

    /// Bind an incoming portal inlet connection listener for this router
    pub async fn bind_inlet(
        &self,
        onward_route: impl Into<Route>,
        addr: impl Into<SocketAddr>,
    ) -> Result<Address> {
        let socket_addr = addr.into();
        let addr = TcpInletListenProcessor::start(
            &self.ctx,
            onward_route.into(),
            socket_addr,
            Arc::clone(&self.run),
        )
        .await?;

        Ok(addr)
    }

    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_processor(addr).await?;

        Ok(())
    }

    fn resolve_peer(peer: impl Into<String>) -> Result<(SocketAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let hostnames;

        // Try to parse as SocketAddr
        if let Ok(p) = parse_socket_addr(peer_str.clone()) {
            peer_addr = p;
            hostnames = vec![];
        }
        // Try to resolve hostname
        else if let Ok(iter) = peer_str.to_socket_addrs() {
            // FIXME: We only take ipv4 for now
            if let Some(p) = iter.filter(|x| x.is_ipv4()).next() {
                peer_addr = p;
            } else {
                return Err(TcpError::InvalidAddress.into());
            }

            hostnames = vec![peer_str];
        } else {
            return Err(TcpError::InvalidAddress.into());
        }

        Ok((peer_addr, hostnames))
    }

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect(&self, peer: impl Into<String>) -> Result<()> {
        let (peer_addr, hostnames) = Self::resolve_peer(peer)?;

        let pair = WorkerPair::start(&self.ctx, peer_addr, hostnames).await?;
        self.register(&pair).await?;

        Ok(())
    }

    /// Establish an outgoing TCP connection for Portal Outlet
    pub async fn connect_outlet(&self, peer: impl Into<String>) -> Result<Address> {
        let (peer_addr, _) = Self::resolve_peer(peer)?;

        let address = PortalWorkerPair::new_outlet(&self.ctx, peer_addr).await?;

        Ok(address)
    }
}
