//! TCP Transport utilities for Ockam's routing framework
//!
//! The `ockam_node` (or `ockam_node_no_std`) crate sits at the core
//! of the Ockam routing framework, with transport specific
//! abstraction plugins.  This crate implements a TCP connection
//! plugin for this architecture.
//!
//! You can use Ockam's routing mechanism for cryptographic protocols,
//! key lifecycle, credetial exchange, enrollment, etc, without having
//! to worry about the transport specifics.

#![deny(
    missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate tracing;

pub(crate) mod atomic;
mod error;
mod init;
mod listener;
mod receiver;
mod router;
mod sender;

pub use error::TcpError;
pub use init::WorkerPair;
pub use receiver::TcpRecvWorker;
pub use router::{TcpRouter, TcpRouterHandle};
pub use sender::TcpSendWorker;

use ockam_core::lib::net::SocketAddr;
use ockam_core::{Result, ServiceBuilder};
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
pub struct TcpTransport {
    router: TcpRouterHandle,
}

/// TCP address type constant
pub const TCP: u8 = 1;

fn parse_socket_addr(s: impl Into<String>) -> Result<SocketAddr> {
    Ok(s.into().parse().map_err(|_| TcpError::InvalidAddress)?)
}

impl TcpTransport {
    /// Create a new TCP transport and router for the current node
    pub async fn create(ctx: &Context) -> Result<TcpTransport> {
        let router = TcpRouter::register(ctx).await?;

        Ok(Self { router })
    }

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect(&self, peer: impl Into<String>) -> Result<ServiceBuilder> {
        self.router.connect(peer).await
    }

    /// Start listening to incoming connections on an existing transport
    pub async fn listen(&self, bind_addr: impl Into<String>) -> Result<()> {
        let bind_addr = parse_socket_addr(bind_addr)?;
        self.router.bind(bind_addr).await?;
        Ok(())
    }
}
