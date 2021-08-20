use crate::{
    atomic::{self, ArcBool},
    TcpError, TcpRouterHandle, WorkerPair,
};
use async_trait::async_trait;
use ockam_core::{Address, Processor, Result};
use ockam_node::Context;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, trace};

pub(crate) struct TcpListenProcessor {
    inner: TcpListener,
    run: ArcBool,
    router_handle: TcpRouterHandle,
}

impl TcpListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        router_handle: TcpRouterHandle,
        addr: SocketAddr,
        run: ArcBool,
    ) -> Result<()> {
        let waddr = Address::random(0);

        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr).await.map_err(TcpError::from)?;
        let worker = Self {
            inner,
            run,
            router_handle,
        };

        ctx.start_processor(waddr, worker).await?;
        Ok(())
    }
}

#[async_trait]
impl Processor for TcpListenProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        // FIXME: see ArcBool future note
        if atomic::check(&self.run) {
            trace!("Waiting for incoming TCP connection...");

            // Wait for an incoming connection
            let (stream, peer) = self.inner.accept().await.map_err(TcpError::from)?;

            // And spawn a connection worker for it
            let pair = WorkerPair::new_with_stream(ctx, stream, peer, vec![]).await?;

            // Register the connection with the local TcpRouter
            self.router_handle.register(&pair).await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
