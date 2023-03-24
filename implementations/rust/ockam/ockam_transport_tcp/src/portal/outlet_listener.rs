use crate::portal::addresses::{Addresses, PortalType};
use crate::{PortalMessage, TcpOutletTrustOptions, TcpPortalWorker, TcpRegistry};
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
    trust_options: TcpOutletTrustOptions,
}

impl TcpOutletListenWorker {
    /// Create a new `TcpOutletListenWorker`
    fn new(registry: TcpRegistry, peer: SocketAddr, trust_options: TcpOutletTrustOptions) -> Self {
        Self {
            registry,
            peer,
            trust_options,
        }
    }

    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        address: Address,
        peer: SocketAddr,
        trust_options: TcpOutletTrustOptions,
    ) -> Result<()> {
        let access_control = trust_options.incoming_access_control.clone();

        if let Some(consumer_session) = &trust_options.consumer_session {
            consumer_session.sessions.add_consumer(
                &address,
                &consumer_session.session_id,
                consumer_session.session_policy,
            );
        }

        let worker = Self::new(registry, peer, trust_options);
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

        // Check if the Worker that send us this message is a Producer
        // If yes - outlet worker will be added to that session to be able to receive further
        // messages from that Producer
        let session_id = if let Some(consumer_session) = &self.trust_options.consumer_session {
            consumer_session
                .sessions
                .get_session_with_producer(&src_addr)
                .map(|x| x.session_id().clone())
        } else {
            None
        };

        let addresses = Addresses::generate(PortalType::Outlet);

        self.trust_options.setup_session(&addresses, session_id)?;

        TcpPortalWorker::start_new_outlet(
            ctx,
            self.registry.clone(),
            self.peer,
            return_route.clone(),
            addresses.clone(),
            self.trust_options.incoming_access_control.clone(),
        )
        .await?;

        debug!("Created Tcp Outlet at {}", addresses.remote);

        Ok(())
    }
}
