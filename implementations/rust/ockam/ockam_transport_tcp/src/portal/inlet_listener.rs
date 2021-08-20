use crate::atomic::ArcBool;
use crate::{atomic, PortalWorkerPair, TcpError};
use async_trait::async_trait;
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Processor, Result, Route};
use ockam_node::Context;
use tokio::net::TcpListener;
use tracing::debug;

pub(crate) struct TcpInletListenProcessor {
    inner: TcpListener,
    onward_route: Route,
    run: ArcBool,
}

impl TcpInletListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        onward_route: Route,
        addr: SocketAddr,
        run: ArcBool,
    ) -> Result<Address> {
        let waddr = Address::random(0);

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = TcpListener::bind(addr).await.map_err(TcpError::from)?;
        let processor = Self {
            inner,
            onward_route,
            run,
        };

        ctx.start_processor(waddr.clone(), processor).await?;

        Ok(waddr)
    }
}

#[async_trait]
impl Processor for TcpInletListenProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        // FIXME: see ArcBool future note

        if atomic::check(&self.run) {
            let (stream, peer) = self.inner.accept().await.unwrap(); //map_err(TcpError::from)?;
            PortalWorkerPair::new_inlet(ctx, stream, peer, self.onward_route.clone()).await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
