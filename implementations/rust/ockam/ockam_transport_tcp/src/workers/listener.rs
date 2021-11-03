use crate::{TcpRouterHandle, TcpSendWorker};
use ockam_core::async_trait;
use ockam_core::{Address, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, trace};

pub(crate) struct TcpListenProcessor {
    inner: TcpListener,
    router_handle: TcpRouterHandle,
}

impl TcpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        router_handle: TcpRouterHandle,
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
impl Processor for TcpListenProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
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
