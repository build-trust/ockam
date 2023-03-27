mod common;
mod connection;
mod lifecycle;
mod listener;
mod portals;

pub use crate::portal::trust_options::*;

use crate::TcpRegistry;
use ockam_core::AsyncTryClone;
use ockam_node::Context;

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
/// use ockam_transport_tcp::{TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000", TcpListenerTrustOptions::insecure()).await?; // Listen on port 8000
/// tcp.connect("127.0.0.1:5000", TcpConnectionTrustOptions::insecure()).await?; // And connect to port 5000
/// # Ok(()) }
/// ```
///
/// The same `TcpTransport` can also bind to multiple ports.
///
/// ```rust
/// use ockam_transport_tcp::{TcpListenerTrustOptions, TcpTransport};
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let tcp = TcpTransport::create(&ctx).await?;
/// tcp.listen("127.0.0.1:8000", TcpListenerTrustOptions::insecure()).await?; // Listen on port 8000
/// tcp.listen("127.0.0.1:9000", TcpListenerTrustOptions::insecure()).await?; // Listen on port 9000
/// # Ok(()) }
/// ```
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct TcpTransport {
    ctx: Context,
    registry: TcpRegistry,
}
