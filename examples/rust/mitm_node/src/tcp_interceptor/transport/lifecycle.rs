use ockam_core::{Result, TryClone};
use ockam_node::Context;

use crate::tcp_interceptor::{TcpMitmRegistry, TcpMitmTransport};

impl TcpMitmTransport {
    pub fn create(ctx: &Context) -> Result<Self> {
        let tcp = Self {
            ctx: ctx.try_clone()?,
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
