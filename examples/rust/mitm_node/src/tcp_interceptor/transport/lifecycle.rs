use ockam_core::{AsyncTryClone, Result};
use ockam_node::Context;

use crate::tcp_interceptor::{TcpMitmRegistry, TcpMitmTransport};

impl TcpMitmTransport {
    pub async fn create(ctx: &Context) -> Result<Self> {
        let tcp = Self {
            ctx: ctx.async_try_clone().await?,
            registry: Default::default(),
        };
        Ok(tcp)
    }
}

impl TcpMitmTransport {
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    pub fn registry(&self) -> &TcpMitmRegistry {
        &self.registry
    }
}
