use core::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use ockam_core::{Address, Result};
use ockam_node::Context;

use crate::{parse_socket_addr, WebSocketRouter, WebSocketRouterHandle, WS};

/// High level management interface for WebSocket transports.
///
/// Be aware that only one `WebSocketTransport` can exist per node, as it
/// registers itself as a router for the `WS` address type. Multiple
/// calls to [`WebSocketTransport::create`](crate::WebSocketTransport::create)
/// will fail.
///
/// To listen for incoming connections use
/// [`ws.listen()`](crate::WebSocketTransport::listen).
///
/// To register additional connections on an already initialised
/// `WebSocketTransport`, use [`ws.connect()`](crate::WebSocketTransport::connect).
/// This step is optional because the underlying WebSocketRouter is capable of lazily
/// establishing a connection upon arrival of an initial message.
///
/// ```rust
/// use ockam_transport_websocket::WebSocketTransport;
/// # use ockam_core::Result;
/// # use ockam_node::Context;
/// # async fn test(ctx: Context) -> Result<()> {
/// let ws = WebSocketTransport::create(&ctx).await?;
/// ws.listen("127.0.0.1:8000").await?; // Listen on port 8000
/// ws.connect("127.0.0.1:5000").await?; // And connect to port 5000
/// # Ok(()) }
/// ```
///
/// The same `WebSocketTransport` can also bind to multiple ports.
///
/// ```rust
/// use ockam_transport_websocket::WebSocketTransport;
/// # use ockam_core::{Address, Result};
/// # use ockam_node::Context;
/// # async fn test(ctx: Context) -> Result<()> {
/// let ws = WebSocketTransport::create(&ctx).await?;
/// ws.listen("127.0.0.1:8000").await?; // Listen on port 8000
/// ws.listen("127.0.0.1:9000").await?; // Listen on port 9000
/// # Ok(()) }
/// ```
pub struct WebSocketTransport {
    router_handle: WebSocketRouterHandle,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport and router for the current node.
    ///
    /// ```rust
    /// use ockam_transport_websocket::WebSocketTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let ws = WebSocketTransport::create(&ctx).await?;
    /// # Ok(()) }
    /// ```
    pub async fn create(ctx: &Context) -> Result<WebSocketTransport> {
        let router_handle = WebSocketRouter::register(ctx).await?;
        Ok(Self { router_handle })
    }

    /// Establish an outgoing WebSocket connection on an existing transport.
    ///
    /// ```rust
    /// use ockam_transport_websocket::WebSocketTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let ws = WebSocketTransport::create(&ctx).await?;
    /// ws.listen("127.0.0.1:8000").await?; // Listen on port 8000
    /// ws.connect("127.0.0.1:5000").await?; // and connect to port 5000
    /// # Ok(()) }
    /// ```
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        self.router_handle.connect(peer).await
    }

    /// Start listening to incoming connections on an existing transport.
    ///
    /// ```rust
    /// use ockam_transport_websocket::WebSocketTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let ws = WebSocketTransport::create(&ctx).await?;
    /// ws.listen("127.0.0.1:8000").await?;
    /// # Ok(()) }
    pub async fn listen<S: AsRef<str>>(&self, bind_addr: S) -> Result<()> {
        let bind_addr = parse_socket_addr(bind_addr)?;
        self.router_handle.bind(bind_addr).await
    }
}

#[derive(Clone)]
pub(crate) struct WebSocketAddress {
    protocol: String,
    socket_addr: SocketAddr,
}

impl From<WebSocketAddress> for Address {
    fn from(a: WebSocketAddress) -> Self {
        Address::new(WS, a.socket_addr.to_string())
    }
}

impl From<SocketAddr> for WebSocketAddress {
    fn from(socket_addr: SocketAddr) -> Self {
        Self {
            protocol: "ws".to_string(),
            socket_addr,
        }
    }
}

impl From<WebSocketAddress> for SocketAddr {
    fn from(other: WebSocketAddress) -> Self {
        other.socket_addr
    }
}

impl From<&WebSocketAddress> for String {
    fn from(other: &WebSocketAddress) -> Self {
        other.to_string()
    }
}

impl FromStr for WebSocketAddress {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> Result<Self> {
        let socket_addr = parse_socket_addr(s)?;
        Ok(WebSocketAddress::from(socket_addr))
    }
}

impl fmt::Display for WebSocketAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", &self.protocol, &self.socket_addr)
    }
}
