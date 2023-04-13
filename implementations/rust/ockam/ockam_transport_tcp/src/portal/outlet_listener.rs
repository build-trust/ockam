use crate::portal::addresses::{Addresses, PortalType};
use crate::{PortalMessage, TcpOutletOptions, TcpPortalWorker, TcpRegistry};
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, DenyAll, Mailboxes, Result, Routed, Worker};
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
    options: TcpOutletOptions,
}

impl TcpOutletListenWorker {
    /// Create a new `TcpOutletListenWorker`
    fn new(registry: TcpRegistry, peer: SocketAddr, options: TcpOutletOptions) -> Self {
        Self {
            registry,
            peer,
            options,
        }
    }

    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        address: Address,
        peer: SocketAddr,
        options: TcpOutletOptions,
    ) -> Result<()> {
        let access_control = options.incoming_access_control.clone();

        options.setup_flow_control_for_outlet_listener(ctx.flow_controls(), &address);

        let worker = Self::new(registry, peer, options);
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
        let src_addr = msg.src_addr();

        if let PortalMessage::Ping = msg.body() {
        } else {
            return Err(TransportError::Protocol.into());
        }

        let addresses = Addresses::generate(PortalType::Outlet);

        self.options
            .setup_flow_control_for_outlet(ctx.flow_controls(), &addresses, &src_addr);

        TcpPortalWorker::start_new_outlet(
            ctx,
            self.registry.clone(),
            self.peer,
            return_route.clone(),
            addresses.clone(),
            self.options.incoming_access_control.clone(),
        )
        .await?;

        debug!("Created Tcp Outlet at {}", addresses.remote);

        Ok(())
    }
}
