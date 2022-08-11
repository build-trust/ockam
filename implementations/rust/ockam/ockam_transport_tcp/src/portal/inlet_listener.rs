use crate::TcpPortalWorker;
use ockam_core::compat::net::SocketAddr;
use ockam_core::{
    async_trait,
    compat::{boxed::Box, sync::Arc},
};
use ockam_core::{Address, Mailbox, Mailboxes, Processor, Result, Route};
use ockam_node::{Context, ProcessorBuilder};
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::debug;

/// A TCP Portal Inlet listen processor
///
/// TCP Portal Inlet listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_inlet`](crate::TcpTransport::create_inlet).
pub(crate) struct TcpInletListenProcessor {
    inner: TcpListener,
    outlet_listener_route: Route,
    router_address: Address, // TODO @ac for AccessControl
}

impl TcpInletListenProcessor {
    /// Start a new `TcpInletListenProcessor`
    pub(crate) async fn start(
        ctx: &Context,
        outlet_listener_route: Route,
        addr: SocketAddr,
        router_address: Address,
    ) -> Result<(Address, SocketAddr)> {
        let waddr = Address::random_tagged("TcpInletListenProcessor");

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)?;
        let saddr = inner.local_addr().map_err(TransportError::from)?;
        let processor = Self {
            inner,
            outlet_listener_route,
            router_address,
        };

        // @ac 0#TcpInletListenProcessor
        // in:  n/a
        // out: n/a
        let mailbox = Mailbox::new(
            waddr.clone(),
            Arc::new(ockam_core::DenyAll),
            Arc::new(ockam_core::DenyAll),
        );
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), processor)
            .start(ctx)
            .await?;

        Ok((waddr, saddr))
    }
}

#[async_trait]
impl Processor for TcpInletListenProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;
        TcpPortalWorker::start_new_inlet(
            ctx,
            stream,
            peer,
            self.router_address.clone(),
            self.outlet_listener_route.clone(),
        )
        .await?;

        Ok(true)
    }
}
