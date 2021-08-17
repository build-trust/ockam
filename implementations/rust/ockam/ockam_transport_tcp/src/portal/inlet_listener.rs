use crate::atomic::ArcBool;
use crate::{atomic, PortalWorkerPair, TcpError};
use async_trait::async_trait;
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Result, Route, Worker};
use ockam_node::Context;
use tokio::net::TcpListener;
use tracing::debug;

pub(crate) struct TcpInletListenWorker {
    inner: TcpListener,
    onward_route: Route,
    run: ArcBool,
}

impl TcpInletListenWorker {
    pub(crate) async fn start(
        ctx: &Context,
        onward_route: Route,
        addr: SocketAddr,
        run: ArcBool,
    ) -> Result<()> {
        let waddr = Address::random(0);

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = TcpListener::bind(addr).await.map_err(TcpError::from)?;
        let worker = Self {
            inner,
            onward_route,
            run,
        };

        ctx.start_worker(waddr, worker).await?;
        Ok(())
    }
}

#[async_trait]
impl Worker for TcpInletListenWorker {
    type Context = Context;

    // Do not actually listen for messages
    type Message = ();

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // FIXME: see ArcBool future note
        while atomic::check(&self.run) {
            debug!("Waiting for incoming TCP portal connection...");

            // Wait for an incoming connection
            let (stream, peer) = self.inner.accept().await.map_err(TcpError::from)?;

            // And spawn a connection worker for it
            PortalWorkerPair::new_inlet(ctx, stream, peer, self.onward_route.clone()).await?;
        }

        ctx.stop_worker(ctx.address()).await
    }
}
