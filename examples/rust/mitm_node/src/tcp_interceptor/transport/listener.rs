use crate::tcp_interceptor::transport::common::parse_socket_addr;
use crate::tcp_interceptor::{TcpMitmListenProcessor, TcpMitmTransport};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, AsyncTryClone, Result};

impl TcpMitmTransport {
    pub async fn listen(
        &self,
        bind_addr: impl AsRef<str>,
        target_addr: impl AsRef<str>,
    ) -> Result<(SocketAddr, Address)> {
        let bind_addr = parse_socket_addr(bind_addr.as_ref())?;
        let target_addr = parse_socket_addr(target_addr.as_ref())?;
        // Could be different from the bind_addr, e.g., if binding to port 0\
        let (socket_addr, address) = TcpMitmListenProcessor::start(
            &self.ctx,
            self.registry.clone(),
            bind_addr,
            self.async_try_clone().await?,
            target_addr,
        )
        .await?;

        Ok((socket_addr, address))
    }

    /// Interrupt an active TCP listener given its `Address`
    pub async fn stop_listener(&self, address: &Address) -> Result<()> {
        self.ctx.stop_processor(address.clone()).await
    }
}
