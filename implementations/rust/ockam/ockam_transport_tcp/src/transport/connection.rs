use crate::transport::connect;
use crate::workers::{Addresses, TcpRecvProcessor, TcpSendWorker};
use crate::{TcpConnectionMode, TcpConnectionOptions, TcpTransport};
use core::fmt;
use core::fmt::Formatter;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Result};
use ockam_node::Context;
use ockam_transport_core::resolve_peer;
use std::net::SocketAddr;
use tracing::debug;

/// Result of [`TcpTransport::connect`] call.
#[derive(Clone, Debug)]
pub struct TcpConnection {
    sender_address: Address,
    receiver_address: Address,
    socket_address: SocketAddr,
    mode: TcpConnectionMode,
    flow_control_id: FlowControlId,
}

impl fmt::Display for TcpConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Socket: {}, Worker: {}, Processor: {}, FlowId: {}",
            self.socket_address, self.sender_address, self.receiver_address, self.flow_control_id
        )
    }
}

impl From<TcpConnection> for Address {
    fn from(value: TcpConnection) -> Self {
        value.sender_address
    }
}

impl TcpConnection {
    /// Constructor
    pub fn new(
        sender_address: Address,
        receiver_address: Address,
        socket_address: SocketAddr,
        mode: TcpConnectionMode,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            sender_address,
            receiver_address,
            socket_address,
            mode,
            flow_control_id,
        }
    }
    /// Stops the [`TcpConnection`], this method must be called to avoid
    /// leakage of the connection.
    /// Simply dropping this object won't close the connection
    pub async fn stop(&self, context: &Context) -> Result<()> {
        context.stop_worker(self.sender_address.clone()).await
    }
    /// Corresponding [`TcpSendWorker`](super::workers::TcpSendWorker) [`Address`] that can be used
    /// in a route to send messages to the other side of the TCP connection
    pub fn sender_address(&self) -> &Address {
        &self.sender_address
    }
    /// Corresponding [`TcpReceiveProcessor`](super::workers::TcpRecvProcessor) [`Address`]
    pub fn receiver_address(&self) -> &Address {
        &self.receiver_address
    }
    /// Corresponding [`SocketAddr`]
    pub fn socket_address(&self) -> &SocketAddr {
        &self.socket_address
    }
    /// Generated fresh random [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    /// Corresponding [`TcpConnectionMode`]
    pub fn mode(&self) -> TcpConnectionMode {
        self.mode
    }
}

impl TcpTransport {
    /// Establish an outgoing TCP connection.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000", TcpListenerOptions::new()).await?; // Listen on port 8000
    /// let connection = tcp.connect("127.0.0.1:5000", TcpConnectionOptions::new()).await?; // and connect to port 5000
    /// # Ok(()) }
    /// ```
    pub async fn connect(
        &self,
        peer: impl Into<String>,
        options: TcpConnectionOptions,
    ) -> Result<TcpConnection> {
        let peer = peer.into();
        let socket = resolve_peer(peer.clone())?;
        debug!("Connecting to {}", peer.clone());
        let (read_half, write_half) = connect(socket).await?;

        let mode = TcpConnectionMode::Outgoing;
        let addresses = Addresses::generate(mode);

        options.setup_flow_control(self.ctx.flow_controls(), &addresses);
        let flow_control_id = options.flow_control_id.clone();
        let receiver_outgoing_access_control =
            options.create_receiver_outgoing_access_control(self.ctx.flow_controls());

        TcpSendWorker::start(
            &self.ctx,
            self.registry.clone(),
            write_half,
            &addresses,
            socket,
            mode,
            &flow_control_id,
        )
        .await?;

        TcpRecvProcessor::start(
            &self.ctx,
            self.registry.clone(),
            read_half,
            &addresses,
            socket,
            mode,
            &flow_control_id,
            receiver_outgoing_access_control,
        )
        .await?;

        Ok(TcpConnection::new(
            addresses.sender_address().clone(),
            addresses.receiver_address().clone(),
            socket,
            mode,
            flow_control_id,
        ))
    }

    /// Interrupt an active TCP connection given its Sender `Address`
    pub async fn disconnect(&self, address: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(address.into()).await
    }
}
