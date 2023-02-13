use crate::{TcpPortalWorker, TcpRegistry};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{
    async_trait,
    compat::{boxed::Box, sync::Arc},
    DenyAll,
};
use ockam_core::{Address, IncomingAccessControl, Mailboxes, Processor, Result, Route};
use ockam_node::{Context, ProcessorBuilder};
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::{debug, error};

/// A TCP Portal Inlet listen processor
///
/// TCP Portal Inlet listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_inlet`](crate::TcpTransport::create_inlet).
pub(crate) struct TcpInletListenProcessor {
    registry: TcpRegistry,
    inner: TcpListener,
    outlet_listener_route: Route,
    access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpInletListenProcessor {
    /// Start a new `TcpInletListenProcessor`
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        outlet_listener_route: Route,
        addr: SocketAddr,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<(Address, SocketAddr)> {
        let waddr = Address::random_tagged("TcpInletListenProcessor");

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = match TcpListener::bind(addr).await {
            Ok(addr) => addr,
            Err(err) => {
                error!(%addr, %err, "could not bind to address");
                return Err(TransportError::from(err).into());
            }
        };
        let saddr = inner.local_addr().map_err(TransportError::from)?;
        let processor = Self {
            registry,
            inner,
            outlet_listener_route,
            access_control: access_control.clone(),
        };

        ProcessorBuilder::with_mailboxes(
            Mailboxes::main(waddr.clone(), Arc::new(DenyAll), Arc::new(DenyAll)),
            processor,
        )
        .start(ctx)
        .await?;

        Ok((waddr, saddr))
    }
}

#[async_trait]
impl Processor for TcpInletListenProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.add_inlet_listener_processor(&ctx.address());

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_inlet_listener_processor(&ctx.address());

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;
        TcpPortalWorker::start_new_inlet(
            ctx,
            self.registry.clone(),
            stream,
            peer,
            self.outlet_listener_route.clone(),
            self.access_control.clone(),
        )
        .await?;

        Ok(true)
    }
}
