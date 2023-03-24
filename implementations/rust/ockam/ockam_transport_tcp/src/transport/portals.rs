use crate::portal::TcpInletListenProcessor;
use crate::transport::common::{parse_socket_addr, resolve_peer};
use crate::{TcpInletTrustOptions, TcpOutletListenWorker, TcpOutletTrustOptions, TcpTransport};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Result, Route};

impl TcpTransport {
    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route_path = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route_path, TcpInletTrustOptions::new()).await?;
    /// # tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_inlet(
        &self,
        bind_addr: impl Into<String>,
        outlet_route: impl Into<Route>,
        trust_options: TcpInletTrustOptions,
    ) -> Result<(SocketAddr, Address)> {
        let socket_addr = parse_socket_addr(&bind_addr.into())?;
        TcpInletListenProcessor::start(
            &self.ctx,
            self.registry.clone(),
            outlet_route.into(),
            socket_addr,
            trust_options,
        )
        .await
    }

    /// Stop inlet at addr
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route, TcpInletTrustOptions::new()).await?;
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
    /// use ockam_transport_tcp::{TcpOutletTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", "localhost:9000", TcpOutletTrustOptions::new()).await?;
    /// # tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_outlet(
        &self,
        address: impl Into<Address>,
        peer: impl Into<String>,
        trust_options: TcpOutletTrustOptions,
    ) -> Result<()> {
        // Resolve peer address
        let peer_addr = resolve_peer(peer.into())?;
        TcpOutletListenWorker::start(
            &self.ctx,
            self.registry.clone(),
            address.into(),
            peer_addr,
            trust_options,
        )
        .await?;

        Ok(())
    }

    /// Stop outlet at addr
    /// ```rust
    /// use ockam_transport_tcp::{TcpOutletTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// const TARGET_PEER: &str = "127.0.0.1:5000";
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", TARGET_PEER, TcpOutletTrustOptions::new()).await?;
    /// tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}
