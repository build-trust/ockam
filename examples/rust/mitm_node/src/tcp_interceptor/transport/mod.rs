mod common;
mod lifecycle;
mod listener;

use crate::tcp_interceptor::TcpMitmRegistry;
use ockam_core::AsyncTryClone;
use ockam_node::Context;

#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct TcpMitmTransport {
    ctx: Context,
    registry: TcpMitmRegistry,
}
