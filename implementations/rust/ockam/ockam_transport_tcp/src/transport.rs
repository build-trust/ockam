use crate::{parse_socket_addr, TcpOutletListenWorker, TcpRouter, TcpRouterHandle};
use ockam_core::{Address, Result, Route};
use ockam_node::Context;

/// High level management interface for TCP transports
///
/// Be aware that only one `TcpTransport` can exist per node, as it
/// registers itself as a router for the `TCP` address type.  Multiple
/// calls to [`TcpTransport::create`](crate::TcpTransport::create)
/// will fail.
///
/// To register additional connections on an already initialised
/// `TcpTransport`, use
/// [`tcp.connect()`](crate::TcpTransport::connect).  To listen for
/// incoming connections use
/// [`tcp.listen()`](crate::TcpTransport::listen)
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
/// # use ockam_transport_tcp::TcpTransport;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000").await?; // Listen on port 8000
/// tcp.listen("127.0.0.1:9000").await?; // Listen on port 9000
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct TcpTransport {
    router_handle: TcpRouterHandle,
}

impl TcpTransport {
    /// Create a new TCP transport and router for the current node
    pub async fn create(ctx: &Context) -> Result<Self> {
        let router = TcpRouter::register(ctx).await?;

        Ok(Self {
            router_handle: router,
        })
    }

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect(&self, peer: impl Into<String>) -> Result<()> {
        self.router_handle.connect(peer).await
    }

    /// Start listening to incoming connections on an existing transport
    pub async fn listen(&self, bind_addr: impl Into<String>) -> Result<()> {
        let bind_addr = parse_socket_addr(bind_addr)?;
        self.router_handle.bind(bind_addr).await?;
        Ok(())
    }
}

impl TcpTransport {
    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using onward_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    pub async fn create_inlet(
        &self,
        bind_addr: impl Into<String>,
        onward_route: impl Into<Route>,
    ) -> Result<Address> {
        let bind_addr = parse_socket_addr(bind_addr)?;
        let addr = self
            .router_handle
            .bind_inlet(onward_route, bind_addr)
            .await?;

        Ok(addr)
    }

    /// Stop inlet at addr
    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.router_handle.stop_inlet(addr).await?;

        Ok(())
    }

    /// Create Tcp Outlet Listener at address, that connects to peer using Tcp, transforms Ockam Messages
    /// received from Inlet into stream and sends it to peer Tcp stream. Outlet is bidirectional:
    /// Tcp stream received from peer is transformed into Ockam Routable Messages and sent
    /// to Inlet using return route.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    pub async fn create_outlet(
        &self,
        address: impl Into<Address>,
        peer: impl Into<String>,
    ) -> Result<()> {
        TcpOutletListenWorker::start(&self.router_handle, address.into(), peer.into()).await?;

        Ok(())
    }
}
