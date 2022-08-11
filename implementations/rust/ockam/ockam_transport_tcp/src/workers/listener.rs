use crate::{TcpRouterHandle, TcpSendWorker};
use ockam_core::{
    async_trait,
    compat::{net::SocketAddr, sync::Arc},
    AsyncTryClone,
};
use ockam_core::{Address, Mailbox, Mailboxes, Processor, Result};
use ockam_node::{Context, ProcessorBuilder};
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::{debug, trace};

/// A TCP Listen processor
///
/// TCP listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::listen`](crate::TcpTransport::listen).
pub(crate) struct TcpListenProcessor {
    inner: TcpListener,
    router_handle: TcpRouterHandle,
}

impl TcpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        router_handle: TcpRouterHandle,
        addr: SocketAddr,
    ) -> Result<SocketAddr> {
        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let saddr = inner.local_addr().map_err(TransportError::from)?;
        let processor = Self {
            inner,
            router_handle,
        };

        // TODO @ac 0#TcpListenProcessor
        // in:  n/a - but it breaks if we set DenyAll - anything inheriting
        //            context from us maybe like TcpSendWorker_tx_addr or DelayedEvent ?
        // out: n/a
        let mailbox = Mailbox::new(
            Address::random_tagged("TcpListenProcessor"),
            Arc::new(ockam_node::access_control::LocalOriginOnly), // DEBUG
            Arc::new(ockam_core::DenyAll),
        );
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
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        debug!("Waiting for incoming TCP connection...");

        // Wait for an incoming connection
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;
        debug!("TCP connection accepted");

        let handle_clone = self.router_handle.async_try_clone().await?;
        // And create a connection worker for it
        let (worker, pair) =
            TcpSendWorker::new_pair(ctx, handle_clone, Some(stream), peer, Vec::new()).await?;

        // Register the connection with the local TcpRouter
        self.router_handle.register(&pair).await?;
        debug!(%peer, "TCP connection registered");

        trace! {
            peer = %peer,
            tx_addr = %pair.tx_addr(),
            int_addr = %worker.internal_addr(),
            "starting tcp connection worker"
        };

        ctx.start_worker(vec![pair.tx_addr(), worker.internal_addr().clone()], worker)
            .await?;

        Ok(true)
    }
}
