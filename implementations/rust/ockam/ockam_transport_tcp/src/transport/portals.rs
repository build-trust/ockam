use crate::portal::TcpInletListenProcessor;
use crate::transport::common::parse_socket_addr;
use crate::{
    portal::TcpOutletListenWorker, HostnamePort, TcpInletOptions, TcpOutletOptions, TcpTransport,
};
use core::fmt::Debug;
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Result, Route};
use tracing::instrument;

impl TcpTransport {
    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route_path = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route_path, TcpInletOptions::new()).await?;
    /// # tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? bind_addr.clone().into(), outlet_route = ? outlet_route.clone()))]
    pub async fn create_inlet(
        &self,
        bind_addr: impl Into<String> + Clone + Debug,
        outlet_route: impl Into<Route> + Clone + Debug,
        options: TcpInletOptions,
    ) -> Result<(SocketAddr, Address)> {
        let socket_addr = parse_socket_addr(&bind_addr.into())?;
        TcpInletListenProcessor::start(
            &self.ctx,
            self.registry.clone(),
            outlet_route.into(),
            socket_addr,
            options,
        )
        .await
    }

    /// Stop inlet at addr
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route, TcpInletOptions::new()).await?;
    /// tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? addr.clone().into()))]
    pub async fn stop_inlet(&self, addr: impl Into<Address> + Clone + Debug) -> Result<()> {
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
    /// use ockam_transport_tcp::{HostnamePort, TcpOutletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", HostnamePort::new("localhost", 9000), TcpOutletOptions::new()).await?;
    /// # tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? address.clone().into(), peer = ? hostname_port.clone()))]
    pub async fn create_outlet(
        &self,
        address: impl Into<Address> + Clone + Debug,
        hostname_port: HostnamePort,
        options: TcpOutletOptions,
    ) -> Result<()> {
        // Resolve peer address as a socket address
        TcpOutletListenWorker::start(
            &self.ctx,
            self.registry.clone(),
            address.into(),
            hostname_port,
            options,
        )
        .await?;

        Ok(())
    }

    /// Create Tcp Outlet Listener at address, that connects to peer using Tcp
    #[instrument(skip(self))]
    pub async fn create_tcp_outlet(
        &self,
        address: Address,
        hostname_port: HostnamePort,
        options: TcpOutletOptions,
    ) -> Result<()> {
        TcpOutletListenWorker::start(
            &self.ctx,
            self.registry.clone(),
            address,
            hostname_port,
            options,
        )
        .await?;

        Ok(())
    }

    /// Stop outlet at addr
    /// ```rust
    /// use ockam_transport_tcp::{HostnamePort, TcpOutletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let target_peer = HostnamePort::new("127.0.0.1", 5000);
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", target_peer, TcpOutletOptions::new()).await?;
    /// tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = % addr.clone().into()))]
    pub async fn stop_outlet(&self, addr: impl Into<Address> + Clone + Debug) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}
