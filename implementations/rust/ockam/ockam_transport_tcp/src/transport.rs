use ockam_core::access_control::AccessControl;
use ockam_core::compat::{boxed::Box, net::SocketAddr};
use ockam_core::{Address, AllowAll, AsyncTryClone, Result, Route};
use ockam_node::Context;
use std::sync::Arc;

use crate::{parse_socket_addr, TcpOutletListenWorker, TcpRouter, TcpRouterHandle};

/// High level management interface for TCP transports
///
/// Be aware that only one `TcpTransport` can exist per node, as it
/// registers itself as a router for the `TCP` address type.  Multiple
/// calls to [`TcpTransport::create`](crate::TcpTransport::create)
/// will fail.
///
/// To listen for incoming connections use
/// [`tcp.listen()`](crate::TcpTransport::listen).
///
/// To register additional connections on an already initialised
/// `TcpTransport`, use [`tcp.connect()`](crate::TcpTransport::connect).
/// This step is optional because the underlying TcpRouter is capable of lazily
/// establishing a connection upon arrival of an initial message.
///
/// ```rust
/// use ockam_transport_tcp::TcpTransport;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000").await?; // Listen on port 8000
/// tcp.connect("127.0.0.1:5000").await?; // And connect to port 5000
/// # Ok(()) }
/// ```
///
/// The same `TcpTransport` can also bind to multiple ports.
///
/// ```rust
/// use ockam_transport_tcp::TcpTransport;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000").await?; // Listen on port 8000
/// tcp.listen("127.0.0.1:9000").await?; // Listen on port 9000
/// # Ok(()) }
/// ```
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct TcpTransport {
    router_handle: TcpRouterHandle,
}

impl TcpTransport {
    /// Create a new TCP transport and router for the current node
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// # Ok(()) }
    /// ```
    pub async fn create(ctx: &Context) -> Result<Self> {
        let router = TcpRouter::register(ctx).await?;

        Ok(Self {
            router_handle: router,
        })
    }

    /// Manually establish an outgoing TCP connection on an existing transport.
    /// This step is optional because the underlying TcpRouter is capable of lazily establishing
    /// a connection upon arrival of the initial message.
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000").await?; // Listen on port 8000
    /// tcp.connect("127.0.0.1:5000").await?; // and connect to port 5000
    /// # Ok(()) }
    /// ```
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<Address> {
        self.router_handle.connect(peer.as_ref()).await
    }

    /// Disconnect from peer
    pub async fn disconnect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        self.router_handle.disconnect(peer.as_ref()).await
    }

    /// Start listening to incoming connections on an existing transport
    ///
    /// Returns the local address that this transport is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out
    /// which port was actually bound.
    ///
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000").await?;
    /// # Ok(()) }
    pub async fn listen<S: AsRef<str>>(&self, bind_addr: S) -> Result<SocketAddr> {
        let bind_addr = parse_socket_addr(bind_addr.as_ref())?;
        self.router_handle.bind(bind_addr).await
    }
}

/// Args to start an Inlet
pub struct InletOptions {
    bind_addr: String,
    outlet_route: Route,
    access_control: Arc<dyn AccessControl>,
}

impl InletOptions {
    // TODO: Generics
    /// Constructor
    pub fn new(
        bind_addr: String,
        outlet_route: Route,
        access_control: Arc<dyn AccessControl>,
    ) -> Self {
        Self {
            bind_addr,
            outlet_route,
            access_control,
        }
    }
}

/// Args to start an Outlet
pub struct OutletOptions {
    address: Address,
    peer: String,
    access_control: Arc<dyn AccessControl>,
}

impl OutletOptions {
    // TODO: Generics
    /// Constructor
    pub fn new(address: Address, peer: String, access_control: Arc<dyn AccessControl>) -> Self {
        Self {
            address,
            peer,
            access_control,
        }
    }
}

impl TcpTransport {
    /// Create an Inlet
    pub async fn create_inlet_extended(
        &self,
        options: InletOptions,
    ) -> Result<(Address, SocketAddr)> {
        let bind_addr = parse_socket_addr(options.bind_addr)?;
        self.router_handle
            .bind_inlet(options.outlet_route, bind_addr, options.access_control)
            .await
    }

    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpTransport, TCP};
    /// # use ockam_node::Context;
    /// # use ockam_core::{Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let hop_addr = "INTERMEDIARY_HOP:8000";
    /// let route_path = route![(TCP, hop_addr), "outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route_path).await?;
    /// # tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_inlet(
        &self,
        bind_addr: impl Into<String>,
        outlet_route: impl Into<Route>,
    ) -> Result<(Address, SocketAddr)> {
        let options = InletOptions::new(bind_addr.into(), outlet_route.into(), Arc::new(AllowAll));

        self.create_inlet_extended(options).await
    }

    /// Stop inlet at addr
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpTransport, TCP};
    /// # use ockam_node::Context;
    /// # use ockam_core::{Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let hop_addr = "INTERMEDIARY_HOP:8000";
    /// let route = route![(TCP, hop_addr), "outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route).await?;
    /// tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.router_handle.stop_inlet(addr).await?;

        Ok(())
    }

    /// Create an Outlet
    pub async fn create_outlet_extended(&self, options: OutletOptions) -> Result<()> {
        let worker = TcpOutletListenWorker::new(options.peer, options.access_control);
        self.router_handle
            .ctx()
            .start_worker(options.address, worker)
            .await?;

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
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", "localhost:9000").await?;
    /// # tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn create_outlet(
        &self,
        address: impl Into<Address>,
        peer: impl Into<String>,
    ) -> Result<()> {
        let options = OutletOptions::new(address.into(), peer.into(), Arc::new(AllowAll));

        self.create_outlet_extended(options).await
    }

    /// Stop outlet at addr
    /// ```rust
    /// use ockam_transport_tcp::TcpTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// const TARGET_PEER: &str = "127.0.0.1:5000";
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", TARGET_PEER).await?;
    /// tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.router_handle.stop_outlet(addr).await?;
        Ok(())
    }
}
