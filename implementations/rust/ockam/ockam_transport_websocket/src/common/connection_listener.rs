use ockam_core::lib::net::SocketAddr;
use ockam_core::{async_trait, Address, Result, Worker};
use ockam_node::Context;

use crate::common::TransportNode;

#[async_trait::async_trait]
pub trait ConnectionListenerWorker: Worker {
    type Transport: TransportNode;

    async fn start(ctx: &Context, addr: SocketAddr, router: Address) -> Result<()>;
}
