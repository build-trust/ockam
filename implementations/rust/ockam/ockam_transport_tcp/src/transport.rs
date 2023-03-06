use ockam_core::access_control::IncomingAccessControl;
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{Address, AsyncTryClone, Result, Route};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::portal::TcpInletListenProcessor;
use crate::workers::{
    Addresses, ConnectionRole, TcpListenProcessor, TcpRecvProcessor, TcpSendWorker,
};
use crate::{
    TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpOutletListenWorker, TcpRegistry,
};

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.tcp";

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
    ctx: Context,
    registry: TcpRegistry,
}

impl TcpTransport {
    /// Getter
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
    /// Registry of all active connections
    pub fn registry(&self) -> &TcpRegistry {
        &self.registry
    }
}

impl TcpTransport {
    /// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
    fn resolve_peer(peer: String) -> Result<SocketAddr> {
        // Try to parse as SocketAddr
        if let Ok(p) = parse_socket_addr(&peer) {
            return Ok(p);
        }

        // Try to resolve hostname
        if let Ok(mut iter) = peer.to_socket_addrs() {
            // Prefer ip4
            if let Some(p) = iter.find(|x| x.is_ipv4()) {
                return Ok(p);
            }
            if let Some(p) = iter.find(|x| x.is_ipv6()) {
                return Ok(p);
            }
        }

        // Nothing worked, return an error
        Err(TransportError::InvalidAddress.into())
    }
}

impl TcpTransport {
    /// Create a TCP transport
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
        Ok(Self {
            ctx: ctx.async_try_clone().await?,
            registry: TcpRegistry::default(),
        })
    }

    /// Establish an outgoing TCP connection.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000", TcpListenerTrustOptions::new()).await?; // Listen on port 8000
    /// let addr = tcp.connect("127.0.0.1:5000", TcpConnectionTrustOptions::new()).await?; // and connect to port 5000
    /// # Ok(()) }
    /// ```
    pub async fn connect(
        &self,
        peer: impl Into<String>,
        trust_options: TcpConnectionTrustOptions,
    ) -> Result<Address> {
        // Resolve peer address
        let socket = Self::resolve_peer(peer.into())?;

        let (read_half, write_half) = TcpSendWorker::connect(socket).await?;

        let access_control = trust_options.access_control();

        let addresses = Addresses::generate(ConnectionRole::Initiator);

        TcpSendWorker::start(
            &self.ctx,
            self.registry.clone(),
            write_half,
            &addresses,
            socket,
            access_control.sender_incoming_access_control,
        )
        .await?;

        TcpRecvProcessor::start(
            &self.ctx,
            self.registry.clone(),
            read_half,
            &addresses,
            socket,
            access_control.receiver_outgoing_access_control,
            access_control.fresh_session_id,
        )
        .await?;

        Ok(addresses.sender_address().clone())
    }

    /// Start listening to incoming connections on an existing transport
    ///
    /// Returns the local address that this transport is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out
    /// which port was actually bound.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpListenerTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000", TcpListenerTrustOptions::new()).await?;
    /// # Ok(()) }
    pub async fn listen(
        &self,
        bind_addr: impl AsRef<str>,
        trust_options: TcpListenerTrustOptions,
    ) -> Result<(SocketAddr, Address)> {
        let bind_addr = parse_socket_addr(bind_addr.as_ref())?;
        // Could be different from the bind_addr, e.g., if binding to port 0\
        let (socket_addr, address) =
            TcpListenProcessor::start(&self.ctx, self.registry.clone(), bind_addr, trust_options)
                .await?;

        Ok((socket_addr, address))
    }

    /// Interrupt an active TCP connection given its `Address`
    pub async fn disconnect(&self, address: &Address) -> Result<()> {
        self.ctx.stop_worker(address.clone()).await
    }

    /// Interrupt an active TCP listener given its `Address`
    pub async fn stop_listener(&self, address: &Address) -> Result<()> {
        self.ctx.stop_processor(address.clone()).await
    }
}

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
        let peer_addr = Self::resolve_peer(peer)?;
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

fn parse_socket_addr(s: &str) -> Result<SocketAddr> {
    Ok(s.parse().map_err(|_| TransportError::InvalidAddress)?)
}

#[cfg(test)]
mod test {
    use core::fmt::Debug;
    use ockam_core::{Error, Result};
    use ockam_transport_core::TransportError;

    use crate::transport::parse_socket_addr;

    fn assert_transport_error<T>(result: Result<T>, error: TransportError)
    where
        T: Debug,
    {
        let invalid_address_error: Error = error.into();
        assert_eq!(result.unwrap_err().code(), invalid_address_error.code())
    }

    #[test]
    fn test_parse_socket_address() {
        let result = parse_socket_addr("hostname:port");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("example.com");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("example.com:80");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:port");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.1:80");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:65536");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:0");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:80");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:8080");
        assert!(result.is_ok());
    }
}
