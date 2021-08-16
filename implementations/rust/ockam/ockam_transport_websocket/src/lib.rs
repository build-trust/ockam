//! WebSocket Transport utilities for Ockam's routing framework
//!
//! The `ockam_node` (or `ockam_node_no_std`) crate sits at the core
//! of the Ockam routing framework, with transport specific
//! abstraction plugins.  This crate implements a WebSocket connection
//! plugin for this architecture.
//!
//! You can use Ockam's routing mechanism for cryptographic protocols,
//! key lifecycle, credetial exchange, enrollment, etc, without having
//! to worry about the transport specifics.
//!
#![deny(
// missing_docs,
dead_code,
trivial_casts,
trivial_numeric_casts,
unsafe_code,
unused_import_braces,
unused_qualifications
)]

#[macro_use]
extern crate tracing;

use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use ockam_core::{Address, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;

pub use error::WebSocketError;
pub use init::WorkerPair;
pub use receiver::WebSocketRecvWorker;
pub use sender::WebSocketSendWorker;

use crate::router::{WebSocketRouter, WebSocketRouterHandle};

mod atomic;
mod error;
#[cfg(test)]
mod error_test;
mod init;
mod listener;
mod receiver;
mod router;
mod sender;

/// High level management interface for WebSocket transports
///
/// Be aware that only one `WebSocketTransport` can exist per node, as it
/// registers itself as a router for the `WS` address type. Multiple
/// calls to [`WebSocketTransport::create`](crate::WebSocketTransport::create)
/// will fail.
///
/// To register additional connections on an already initialised
/// `WebSocketTransport`, use
/// [`ws.connect()`](crate::WebSocketTransport::connect). To listen for
/// incoming connections use
/// [`ws.listen()`](crate::WebSocketTransport::listen)
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
/// # use ockam_transport_websocket::WebSocketTransport;
/// # use ockam_core::{Address, Result};
/// # use ockam_node::Context;
/// # async fn test(ctx: Context) -> Result<()> {
/// let ws = WebSocketTransport::create(&ctx).await?;
/// ws.listen("127.0.0.1:8000").await?; // Listen on port 8000
/// ws.listen("127.0.0.1:9000").await?; // Listen on port 9000
/// # Ok(()) }
/// ```
pub struct WebSocketTransport {
    ctx: Context,
    router: WebSocketRouterHandle,
}

/// WebSocket address type constant
pub const WS: u8 = 2;

fn parse_socket_addr<S: Into<String>>(s: S) -> Result<SocketAddr> {
    Ok(s.into()
        .parse()
        .map_err(|_| TransportError::InvalidAddress)?)
}

impl WebSocketTransport {
    /// Create a new WebSocket transport and router for the current node
    pub async fn create(ctx: &Context) -> Result<WebSocketTransport> {
        let addr = Address::random(0);
        let ctx = ctx.new_context(addr).await?;

        let router = {
            let ctx_addr = Address::random(0);
            let ctx = ctx.new_context(ctx_addr).await?;

            let addr = Address::random(0);
            WebSocketRouter::register(ctx, addr).await?
        };

        Ok(Self { ctx, router })
    }

    /// Establish an outgoing WebSocket connection on an existing transport
    pub async fn connect<S: Into<String>>(&self, peer: S) -> Result<()> {
        let peer = WebSocketAddr::from_str(&peer.into())?;
        init::start_connection(&self.ctx, &self.router, peer).await?;
        Ok(())
    }

    /// Start listening to incoming connections on an existing transport
    pub async fn listen<S: Into<String>>(&self, bind_addr: S) -> Result<()> {
        let bind_addr = parse_socket_addr(bind_addr)?;
        self.router.bind(bind_addr).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct WebSocketAddr {
    protocol: String,
    socket_addr: SocketAddr,
}

impl From<SocketAddr> for WebSocketAddr {
    fn from(socket_addr: SocketAddr) -> Self {
        Self {
            protocol: "ws".to_string(),
            socket_addr,
        }
    }
}

impl From<WebSocketAddr> for SocketAddr {
    fn from(other: WebSocketAddr) -> Self {
        other.socket_addr
    }
}

impl From<&WebSocketAddr> for String {
    fn from(other: &WebSocketAddr) -> Self {
        other.to_string()
    }
}

impl FromStr for WebSocketAddr {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        let socket_addr = parse_socket_addr(s)?;
        Ok(WebSocketAddr::from(socket_addr))
    }
}

impl fmt::Display for WebSocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", &self.protocol, &self.socket_addr)
    }
}
