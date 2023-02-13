use crate::{PortalMessage, TcpPortalWorker, TcpRegistry};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    async_trait, Address, DenyAll, IncomingAccessControl, Mailboxes, Result, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tracing::debug;

/// A TCP Portal Outlet listen worker
///
/// TCP Portal Outlet listen workers are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_outlet`](crate::TcpTransport::create_outlet).
pub(crate) struct TcpOutletListenWorker {
    registry: TcpRegistry,
    peer: SocketAddr,
    access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpOutletListenWorker {
    /// Create a new `TcpOutletListenWorker`
    pub(crate) fn new(
        registry: TcpRegistry,
        peer: SocketAddr,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        Self {
            registry,
            peer,
            access_control,
        }
    }

    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        address: Address,
        peer: SocketAddr,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Result<()> {
        let worker = Self::new(registry, peer, access_control.clone());
        WorkerBuilder::with_mailboxes(
            Mailboxes::main(address, access_control, Arc::new(DenyAll)),
            worker,
        )
        .start(ctx)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpOutletListenWorker {
    type Context = Context;
    type Message = PortalMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.add_outlet_listener_worker(&ctx.address());

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_outlet_listener_worker(&ctx.address());

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        if let PortalMessage::Ping = msg.body() {
        } else {
            return Err(TransportError::Protocol.into());
        }

        let address = TcpPortalWorker::start_new_outlet(
            ctx,
            self.registry.clone(),
            self.peer,
            return_route.clone(),
            self.access_control.clone(),
        )
        .await?;

        debug!("Created Tcp Outlet at {}", &address);

        Ok(())
    }
}
