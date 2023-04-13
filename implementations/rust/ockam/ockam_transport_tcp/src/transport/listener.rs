use crate::transport::common::parse_socket_addr;
use crate::workers::TcpListenProcessor;
use crate::{TcpListenerOptions, TcpTransport};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Result};

impl TcpTransport {
    /// Start listening to incoming connections on an existing transport
    ///
    /// Returns the local address that this transport is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out
    /// which port was actually bound.
    ///
    /// ```rust
    /// use ockam_core::flow_control::FlowControls;
    /// use ockam_transport_tcp::{TcpListenerOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// let flow_control_id = FlowControls::generate_id();
    /// tcp.listen("127.0.0.1:8000", TcpListenerOptions::new(&flow_control_id)).await?;
    /// # Ok(()) }
    pub async fn listen(
        &self,
        bind_addr: impl AsRef<str>,
        options: TcpListenerOptions,
    ) -> Result<(SocketAddr, Address)> {
        let bind_addr = parse_socket_addr(bind_addr.as_ref())?;
        // Could be different from the bind_addr, e.g., if binding to port 0\
        let (socket_addr, address) =
            TcpListenProcessor::start(&self.ctx, self.registry.clone(), bind_addr, options).await?;

        Ok((socket_addr, address))
    }

    /// Interrupt an active TCP listener given its `Address`
    pub async fn stop_listener(&self, address: &Address) -> Result<()> {
        self.ctx.stop_processor(address.clone()).await
    }
}
