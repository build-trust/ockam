use crate::workers::TcpListenProcessor;
use crate::{TcpListenerOptions, TcpTransport};
use core::fmt;
use core::fmt::Formatter;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Result};
use ockam_transport_core::parse_socket_addr;
use std::net::SocketAddr;

/// Result of [`TcpTransport::listen`] call.
#[derive(Clone, Debug)]
pub struct TcpListener {
    processor_address: Address,
    socket_address: SocketAddr,
    flow_control_id: FlowControlId,
}

impl fmt::Display for TcpListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Socket: {}, Processor: {}, FlowId: {}",
            self.socket_address, self.processor_address, self.flow_control_id
        )
    }
}

impl TcpListener {
    /// Constructor
    pub fn new(
        processor_address: Address,
        socket_address: SocketAddr,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            processor_address,
            socket_address,
            flow_control_id,
        }
    }
    /// Corresponding Worker [`Address`] that can be used to stop the Listener
    pub fn processor_address(&self) -> &Address {
        &self.processor_address
    }
    /// Corresponding [`SocketAddr`]
    pub fn socket_address(&self) -> &SocketAddr {
        &self.socket_address
    }
    /// Corresponding [`SocketAddr`] in String format
    pub fn socket_string(&self) -> String {
        self.socket_address.to_string()
    }
    /// Generated fresh random [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}

impl TcpTransport {
    /// Start listening to incoming connections on an existing transport
    ///
    /// Returns the local address that this transport is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out
    /// which port was actually bound.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpListenerOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000", TcpListenerOptions::new()).await?;
    /// # Ok(()) }
    pub async fn listen(
        &self,
        bind_addr: impl AsRef<str>,
        options: TcpListenerOptions,
    ) -> Result<TcpListener> {
        let flow_control_id = options.flow_control_id.clone();
        let bind_addr = parse_socket_addr(bind_addr.as_ref())?;
        // Could be different from the bind_addr, e.g., if binding to port 0\
        let (socket_addr, address) =
            TcpListenProcessor::start(&self.ctx, self.registry.clone(), bind_addr, options).await?;

        Ok(TcpListener::new(address, socket_addr, flow_control_id))
    }

    /// Interrupt an active TCP listener given its `Address`
    pub async fn stop_listener(&self, address: &Address) -> Result<()> {
        self.ctx.stop_processor(address.clone()).await
    }
}
