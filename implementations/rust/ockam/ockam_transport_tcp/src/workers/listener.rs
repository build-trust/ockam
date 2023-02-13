use crate::{TcpRegistry, TcpSendWorker};
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    IncomingAccessControl, OutgoingAccessControl,
};
use ockam_core::{Address, Mailbox, Mailboxes, Processor, Result};
use ockam_node::{Context, ProcessorBuilder};
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
    sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

impl TcpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        addr: SocketAddr,
        sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
        receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Result<SocketAddr> {
        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let saddr = inner.local_addr().map_err(TransportError::from)?;
        let processor = Self {
            registry,
            inner,
            sender_incoming_access_control,
            receiver_outgoing_access_control,
        };

        let mailbox = Mailbox::deny_all(Address::random_tagged("TcpListenProcessor"));
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), processor)
            .start(ctx)
            .await?;

        Ok(saddr)
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

        // And create a connection worker for it
        let _sender_worker_address = TcpSendWorker::start(
            ctx,
            self.registry.clone(),
            Some(stream),
            peer,
            self.sender_incoming_access_control.clone(),
            self.receiver_outgoing_access_control.clone(),
        )
        .await?;

        // TODO: Add to registry

        Ok(true)
    }
}
