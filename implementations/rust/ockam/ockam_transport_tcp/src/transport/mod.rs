mod common;
mod connection;
mod lifecycle;
mod listener;
mod portals;

pub use crate::portal::options::*;

use crate::TcpRegistry;
use ockam_core::{async_trait, AsyncTryClone, Result};
use ockam_node::{Context, HasContext};

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
/// use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000", TcpListenerOptions::new()).await?; // Listen on port 8000
/// tcp.connect("127.0.0.1:5000", TcpConnectionOptions::new()).await?; // And connect to port 5000
/// # Ok(()) }
/// ```
///
/// The same `TcpTransport` can also bind to multiple ports.
///
/// ```rust
/// use ockam_transport_tcp::{TcpListenerOptions, TcpTransport};
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000", TcpListenerOptions::new()).await?; // Listen on port 8000
/// tcp.listen("127.0.0.1:9000", TcpListenerOptions::new()).await?; // Listen on port 9000
/// # Ok(()) }
/// ```
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct TcpTransport {
    ctx: Context,
    registry: TcpRegistry,
}

/// This trait adds a `create_tcp_transport` method to any struct returning a Context.
/// This is the case for an ockam::Node, so you can write `node.create_tcp_transport()`
#[async_trait]
pub trait TcpTransportExtension: HasContext {
    /// Create a TCP transport
    async fn create_tcp_transport(&self) -> Result<TcpTransport> {
        TcpTransport::create(self.get_context()).await
    }
}

impl<A: HasContext> TcpTransportExtension for A {}
