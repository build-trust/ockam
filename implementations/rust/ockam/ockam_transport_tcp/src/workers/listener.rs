use crate::workers::{Addresses, ConnectionRole, TcpRecvProcessor};
use crate::{TcpListenerTrustOptions, TcpRegistry, TcpSendWorker};
use ockam_core::{async_trait, compat::net::SocketAddr, DenyAll};
use ockam_core::{Address, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::debug;

/// A TCP Listen processor
///
/// TCP listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::listen`](crate::TcpTransport::listen).
pub(crate) struct TcpListenProcessor {
    registry: TcpRegistry,
    inner: TcpListener,
    trust_options: TcpListenerTrustOptions,
}

impl TcpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        addr: SocketAddr,
        trust_options: TcpListenerTrustOptions,
    ) -> Result<(SocketAddr, Address)> {
        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let saddr = inner.local_addr().map_err(TransportError::from)?;
        let processor = Self {
            registry,
            inner,
            trust_options,
        };

        let address = Address::random_tagged("TcpListenProcessor");
        ctx.start_processor(address.clone(), processor, DenyAll, DenyAll)
            .await?;

        Ok((saddr, address))
    }
}

#[async_trait]
impl Processor for TcpListenProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        self.registry.add_listener_processor(&ctx.address());

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_listener_processor(&ctx.address());

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        debug!("Waiting for incoming TCP connection...");

        // Wait for an incoming connection
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;
        debug!("TCP connection accepted");

        let access_control = self.trust_options.access_control();

        let addresses = Addresses::generate(ConnectionRole::Responder);

        let (read_half, write_half) = stream.into_split();

        // Worker to receive messages from the Node and send them over the wire
        TcpSendWorker::start(
            ctx,
            self.registry.clone(),
            write_half,
            &addresses,
            peer,
            access_control.sender_incoming_access_control,
        )
        .await?;

        // Processor to receive messages over the wire and forward them to the node
        TcpRecvProcessor::start(
            ctx,
            self.registry.clone(),
            read_half,
            &addresses,
            peer,
            access_control.receiver_outgoing_access_control,
            // This session_id (if present) will be added to messages' LocalInfo
            access_control.session_id,
        )
        .await?;

        Ok(true)
    }
}
