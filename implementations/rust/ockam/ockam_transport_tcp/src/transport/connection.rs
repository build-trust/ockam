use crate::transport::common::{resolve_peer, TcpConnection};
use crate::transport::connect;
use crate::workers::{Addresses, TcpRecvProcessor, TcpSendWorker};
use crate::{TcpConnectionMode, TcpConnectionOptions, TcpTransport};
use ockam_core::{Address, Result};
use ockam_node::HostnamePort;
use tracing::debug;

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
        let socket_address = resolve_peer(peer.clone())?;
        let hostname_port = HostnamePort::from_socket_addr(socket_address);
        debug!("Connecting to {}", peer.clone());
        let (read_half, write_half) = connect(&hostname_port).await?;

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
            socket_address,
            mode,
            &flow_control_id,
        )
        .await?;

        TcpRecvProcessor::start(
            &self.ctx,
            self.registry.clone(),
            read_half,
            &addresses,
            socket_address,
            mode,
            &flow_control_id,
            receiver_outgoing_access_control,
        )
        .await?;

        Ok(TcpConnection::new(
            addresses.sender_address().clone(),
            addresses.receiver_address().clone(),
            socket_address,
            mode,
            flow_control_id,
        ))
    }

    /// Interrupt an active TCP connection given its Sender `Address`
    pub async fn disconnect(&self, address: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(address.into()).await
    }
}
