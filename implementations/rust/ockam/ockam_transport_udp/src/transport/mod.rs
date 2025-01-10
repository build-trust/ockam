mod bind;
mod lifecycle;
mod puncture;

pub use bind::*;

use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Result};
use ockam_node::{Context, HasContext};

/// UDP Transport
#[derive(Clone, Debug)]
pub struct UdpTransport {
    ctx: Arc<Context>,
    // TODO: Add registry,
}

/// This trait adds a `create_udp_transport` method to any struct returning a Context.
/// This is the case for an ockam::Node, so you can write `node.create_udp_transport()`
#[async_trait]
pub trait UdpTransportExtension: HasContext {
    /// Create a UDP transport
    async fn create_udp_transport(&self) -> Result<UdpTransport> {
        UdpTransport::create(self.get_context())
    }
}

impl<A: HasContext> UdpTransportExtension for A {}
