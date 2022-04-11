use crate::TcpPortalWorker;
use ockam_core::compat::net::SocketAddr;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, Processor, Result, Route};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::debug;

pub(crate) struct TcpInletListenProcessor {
    inner: TcpListener,
    outlet_listener_route: Route,
}

impl TcpInletListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        outlet_listener_route: Route,
        addr: SocketAddr,
    ) -> Result<Address> {
        let waddr = Address::random_local();

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let processor = Self {
            inner,
            outlet_listener_route,
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
        TcpPortalWorker::start_new_inlet(ctx, stream, peer, self.outlet_listener_route.clone())
            .await?;

        Ok(true)
    }
}
