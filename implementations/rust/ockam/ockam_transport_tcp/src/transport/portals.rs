use crate::portal::TcpInletListenProcessor;
use crate::transport::common::{parse_socket_addr, resolve_peer};
use crate::{TcpOutletListenWorker, TcpTransport};
use ockam_core::access_control::IncomingAccessControl;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Result, Route};

impl TcpTransport {
    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route_path = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route_path, AllowAll).await?;
    /// # tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_inlet(
        &self,
        bind_addr: impl Into<String>,
        outlet_route: impl Into<Route>,
        access_control: impl IncomingAccessControl,
    ) -> Result<(Address, SocketAddr)> {
        self.create_inlet_impl(
            bind_addr.into(),
            outlet_route.into(),
            Arc::new(access_control),
        )
        .await
    }

    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    pub async fn create_inlet_impl(
        &self,
        bind_addr: String,
        outlet_route: Route,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<(Address, SocketAddr)> {
        let socket_addr = parse_socket_addr(&bind_addr)?;
        TcpInletListenProcessor::start(
            &self.ctx,
            self.registry.clone(),
            outlet_route,
            socket_addr,
            access_control,
        )
        .await
    }

    /// Stop inlet at addr
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route, AllowAll).await?;
    /// tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_processor(addr).await?;

        Ok(())
    }

    /// Create Tcp Outlet Listener at address, that connects to peer using Tcp, transforms Ockam Messages
    /// received from Inlet into stream and sends it to peer Tcp stream. Outlet is bidirectional:
    /// Tcp stream received from peer is transformed into Ockam Routable Messages and sent
    /// to Inlet using return route.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", "localhost:9000", AllowAll).await?;
    /// # tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_outlet(
        &self,
        address: impl Into<Address>,
        peer: impl Into<String>,
        access_control: impl IncomingAccessControl,
    ) -> Result<()> {
        self.create_outlet_impl(address.into(), peer.into(), Arc::new(access_control))
            .await
    }

    /// Create Tcp Outlet Listener at address, that connects to peer using Tcp, transforms Ockam Messages
    /// received from Inlet into stream and sends it to peer Tcp stream. Outlet is bidirectional:
    /// Tcp stream received from peer is transformed into Ockam Routable Messages and sent
    /// to Inlet using return route.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    pub async fn create_outlet_impl(
        &self,
        address: Address,
        peer: String,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        // Resolve peer address
        let peer_addr = resolve_peer(peer)?;
        TcpOutletListenWorker::start(
            &self.ctx,
            self.registry.clone(),
            address,
            peer_addr,
            access_control,
        )
        .await?;

        Ok(())
    }

    /// Stop outlet at addr
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// const TARGET_PEER: &str = "127.0.0.1:5000";
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", TARGET_PEER, AllowAll).await?;
    /// tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}
