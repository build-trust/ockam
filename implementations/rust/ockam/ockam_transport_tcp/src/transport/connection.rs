use crate::transport::common::resolve_peer;
use crate::workers::{Addresses, ConnectionRole, TcpRecvProcessor, TcpSendWorker};
use crate::{TcpConnectionTrustOptions, TcpTransport};
use ockam_core::{Address, Result};

impl TcpTransport {
    /// Establish an outgoing TCP connection.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::Result;
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.listen("127.0.0.1:8000", TcpListenerTrustOptions::insecure_test()).await?; // Listen on port 8000
    /// let addr = tcp.connect("127.0.0.1:5000", TcpConnectionTrustOptions::insecure_test()).await?; // and connect to port 5000
    /// # Ok(()) }
    /// ```
    pub async fn connect(
        &self,
        peer: impl Into<String>,
        trust_options: TcpConnectionTrustOptions,
    ) -> Result<Address> {
        // Resolve peer address
        let socket = resolve_peer(peer.into())?;

        let (read_half, write_half) = TcpSendWorker::connect(socket).await?;

        let addresses = Addresses::generate(ConnectionRole::Initiator);

        trust_options.setup_session(&addresses);
        let access_control = trust_options.create_access_control();

        TcpSendWorker::start(
            &self.ctx,
            self.registry.clone(),
            write_half,
            &addresses,
            socket,
            access_control.sender_incoming_access_control,
        )
        .await?;

        TcpRecvProcessor::start(
            &self.ctx,
            self.registry.clone(),
            read_half,
            &addresses,
            socket,
            access_control.receiver_outgoing_access_control,
        )
        .await?;

        Ok(addresses.sender_address().clone())
    }

    /// Interrupt an active TCP connection given its `Address`
    pub async fn disconnect(&self, address: &Address) -> Result<()> {
        self.ctx.stop_worker(address.clone()).await
    }
}
