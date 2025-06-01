mod common;
mod lifecycle;
mod listener;

use crate::tcp_interceptor::TcpMitmRegistry;
use ockam_core::TryClone;
use ockam_node::Context;

#[derive(TryClone)]
#[try_clone(crate = "ockam_core")]
pub struct TcpMitmTransport {
    ctx: Context,
    registry: TcpMitmRegistry,
}
