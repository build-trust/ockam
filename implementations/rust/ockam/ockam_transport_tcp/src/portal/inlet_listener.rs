use crate::PortalWorkerPair;
use ockam_core::async_trait;
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Processor, Result, Route};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::debug;

pub(crate) struct TcpInletListenProcessor {
    inner: TcpListener,
    onward_route: Route,
}

impl TcpInletListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        onward_route: Route,
        addr: SocketAddr,
    ) -> Result<Address> {
        let waddr = Address::random(0);

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let processor = Self {
            inner,
            onward_route,
        };

        ctx.start_processor(waddr.clone(), processor).await?;

        Ok(waddr)
    }
}

#[async_trait]
impl Processor for TcpInletListenProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;
        PortalWorkerPair::new_inlet(ctx, stream, peer, self.onward_route.clone()).await?;

        Ok(true)
    }
}
