use crate::{
    atomic::{self, ArcBool},
    TcpRouterHandle, WorkerPair,
};
use async_trait::async_trait;
use ockam_core::{Address, Result, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, trace};

pub(crate) struct TcpListenWorker {
    inner: TcpListener,
    run: ArcBool,
    router_handle: TcpRouterHandle,
}

impl TcpListenWorker {
    pub(crate) async fn start(
        ctx: &Context,
        router_handle: TcpRouterHandle,
        addr: SocketAddr,
        run: ArcBool,
    ) -> Result<()> {
        let waddr = Address::random(0);

        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let worker = Self {
            inner,
            run,
            router_handle,
        };

        ctx.start_worker(waddr, worker).await?;
        Ok(())
    }
}

#[async_trait]
impl Worker for TcpListenWorker {
    type Context = Context;

    // Do not actually listen for messages
    type Message = ();

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // FIXME: see ArcBool future note
        while atomic::check(&self.run) {
            trace!("Waiting for incoming TCP connection...");

            // Wait for an incoming connection
            let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;

            // And spawn a connection worker for it
            let pair = WorkerPair::new_with_stream(ctx, stream, peer, vec![]).await?;

            // Register the connection with the local TcpRouter
            self.router_handle.register(&pair).await?;
        }

        ctx.stop_worker(ctx.address()).await
    }
}
