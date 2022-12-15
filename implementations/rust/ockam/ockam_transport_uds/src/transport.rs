use std::os::unix::net::SocketAddr;

use ockam_core::{Address, AsyncTryClone, Result};
use ockam_node::Context;

use crate::{
    parse_socket_addr,
    router::{UdsRouter, UdsRouterHandle},
};

/// High level management interface for UDS transports
///
/// Be aware that only one [`UdsTransport`] can exist per node, as it
/// registers itself as a router for the [`UDS`](crate::UDS) address type.  Multiple
/// calls to [`UdsTransport::create`](crate::transport::UdsTransport)
/// will fail.
///
/// To listen for incoming connections use
/// [`uds.listen()`](crate::transport::UdsTransport).
///
/// To register additional connections on an already initialised
/// `UdsTransport`, use [`uds.connect()`](crate::transport::UdsTransport).
/// This step is optional because the underlying UdsRouter is capable of lazily
/// establishing a connection upon arrival of an initial message.
///
/// ```rust
/// use ockam_transport_uds::UdsTransport;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let uds = UdsTransport::create(&ctx).await?;
/// uds.listen("/tmp/example-socket").await?; // Listen on socket `/tmp/example-socket`
/// uds.connect("/tmp/other-socket").await?; // And connect to `/tmp/other-socket`
/// # Ok(()) }
/// ```
///
/// The same `UdsTransport` can also bind to multiple sockets.
///
/// ```rust
/// use ockam_transport_uds::UdsTransport;
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
/// let uds = UdsTransport::create(&ctx).await?;
/// uds.listen("/tmp/socket-one").await?; // Listen on `/tmp/socket-one`
/// uds.listen("/tmp/socket-two").await?; // Listen on `/tmp/socket-two`
/// # Ok(()) }
/// ```
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct UdsTransport {
    router_handle: UdsRouterHandle,
}

impl UdsTransport {
    /// Creates a a UDS Router and registers it with the given node [`Context`]
    pub async fn create(ctx: &Context) -> Result<Self> {
        let router = UdsRouter::register(ctx).await?;

        Ok(Self {
            router_handle: router,
        })
    }

    /// Connects the [`UdsRouterHandle`] to the given socket peer.
    ///
    /// ```rust
    /// use ockam_transport_uds::UdsTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let uds = UdsTransport::create(&ctx).await?;
    /// uds.connect("/tmp/socket-name").await?;
    /// # Ok(()) }
    /// ```
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<Address> {
        self.router_handle.connect(peer.as_ref()).await
    }

    /// Disconnects the [`UdsRouterHandle`] from the given socket peer.
    ///
    /// ```rust
    /// use ockam_transport_uds::UdsTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let uds = UdsTransport::create(&ctx).await?;
    /// uds.connect("/tmp/socket-name").await?;
    ///
    /// uds.disconnect("/tmp/socket-name").await?;
    /// # Ok(()) }
    /// ```
    pub async fn disconnect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        self.router_handle.disconnect(peer.as_ref()).await
    }

    /// Binds the [`UdsRouterHandle`] to listen and accept incomming connection requests to the given socket.
    ///
    /// ```rust
    /// use ockam_transport_uds::UdsTransport;
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let uds = UdsTransport::create(&ctx).await?;
    /// uds.listen("/tmp/socket-name").await?;
    /// # Ok(()) }
    /// ```
    pub async fn listen<S: AsRef<str>>(&self, bind_addr: S) -> Result<SocketAddr> {
        let sock_addr = parse_socket_addr(bind_addr.as_ref())?;
        self.router_handle.bind(sock_addr).await
    }
}
