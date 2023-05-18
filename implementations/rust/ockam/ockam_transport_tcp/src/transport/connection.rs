use crate::transport::common::{resolve_peer, TcpConnection};
use crate::workers::{Addresses, TcpRecvProcessor, TcpSendWorker};
use crate::{TcpConnectionMode, TcpConnectionOptions, TcpTransport};
use ockam_core::{Address, Result};

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
        // Resolve peer address
        let socket = resolve_peer(peer.into())?;

        let (read_half, write_half) = TcpSendWorker::connect(socket).await?;

        let mode = TcpConnectionMode::Outgoing;
        let addresses = Addresses::generate(mode);

        options.setup_flow_control(self.ctx.flow_controls(), &addresses);
        let flow_control_id = options.producer_flow_control_id.clone();
        let access_control = options.create_access_control(self.ctx.flow_controls());

        TcpSendWorker::start(
            &self.ctx,
            self.registry.clone(),
            write_half,
            &addresses,
            socket,
            mode,
            access_control.sender_incoming_access_control,
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
            access_control.receiver_outgoing_access_control,
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

    /// Interrupt an active TCP connection given its `Address`
    pub async fn disconnect(&self, address: &Address) -> Result<()> {
        self.ctx.stop_worker(address.clone()).await
    }
}
