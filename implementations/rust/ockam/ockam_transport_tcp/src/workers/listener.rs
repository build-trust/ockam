use crate::{TcpRouterHandle, TcpSendWorker};
use ockam_core::{async_trait, NodeContext};
use ockam_core::{Address, Processor, Result};
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, trace};

pub(crate) struct TcpListenProcessor<C> {
    inner: TcpListener,
    router_handle: TcpRouterHandle<C>,
}

impl<C: NodeContext> TcpListenProcessor<C> {
    pub(crate) async fn start(
        ctx: &C,
        router_handle: TcpRouterHandle<C>,
        addr: SocketAddr,
    ) -> Result<()> {
        let waddr = Address::random(0);

        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let worker = Self {
            inner,
            router_handle,
        };

        ctx.start_processor(waddr, worker).await?;
        Ok(())
    }
}

#[async_trait]
impl<C: NodeContext> Processor<C> for TcpListenProcessor<C> {
    async fn initialize(&mut self, ctx: &mut C) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME.into()).await
    }

    async fn process(&mut self, ctx: &mut C) -> Result<bool> {
        trace!("Waiting for incoming TCP connection...");

        // Wait for an incoming connection
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;

        // And spawn a connection worker for it
        let pair = TcpSendWorker::start_pair(ctx, Some(stream), peer, vec![]).await?;

        // Register the connection with the local TcpRouter
        self.router_handle.register(&pair).await?;

        Ok(true)
    }
}
