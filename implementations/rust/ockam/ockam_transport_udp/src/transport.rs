use crate::router::{UdpRouter, UdpRouterHandle};
use ockam_core::{async_trait, Result};
use ockam_node::{Context, HasContext};
use ockam_transport_core::TransportError;

/// High level management interface for UDP transport
///
/// A node will have, at most, one UDP transport running.
///
/// This transport only supports IPv4.
pub struct UdpTransport {
    router_handle: UdpRouterHandle,
}

impl UdpTransport {
    /// Create a new UDP transport for the current node
    pub async fn create(ctx: &Context) -> Result<UdpTransport> {
        let router_handle = UdpRouter::register(ctx).await?;
        Ok(Self { router_handle })
    }

    /// Start listening to incoming datagrams on a specified local address
    pub async fn listen<S: AsRef<str>>(&self, bind_addr: S) -> Result<()> {
        let bind_addr = bind_addr
            .as_ref()
            .parse()
            .map_err(|_| TransportError::InvalidAddress)?;
        self.router_handle.listen(bind_addr).await
    }
}

/// This trait adds a `create_udp_transport` method to any struct returning a Context.
/// This is the case for an ockam::Node, so you can write `node.create_udp_transport()`
#[async_trait]
pub trait UdpTransportExtension: HasContext {
    /// Create a UDP transport
    async fn create_udp_transport(&self) -> Result<UdpTransport> {
        UdpTransport::create(&self.context().await?).await
    }
}

impl<A: HasContext> UdpTransportExtension for A {}
