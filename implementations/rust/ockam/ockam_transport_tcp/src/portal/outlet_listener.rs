use crate::portal::addresses::{Addresses, PortalType};
use crate::{portal::TcpPortalWorker, PortalMessage, TcpOutletOptions, TcpRegistry};
use ockam_core::{
    async_trait, Address, DenyAll, NeutralMessage, Result, Routed, SecureChannelLocalInfo, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::{HostnamePort, TransportError};
use tracing::{debug, instrument};

/// A TCP Portal Outlet listen worker
///
/// TCP Portal Outlet listen workers are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_outlet`](crate::TcpTransport::create_outlet).
pub(crate) struct TcpOutletListenWorker {
    registry: TcpRegistry,
    hostname_port: HostnamePort,
    options: TcpOutletOptions,
}

impl TcpOutletListenWorker {
    /// Create a new `TcpOutletListenWorker`
    fn new(registry: TcpRegistry, hostname_port: HostnamePort, options: TcpOutletOptions) -> Self {
        Self {
            registry,
            hostname_port,
            options,
        }
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::start")]
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        address: Address,
        hostname_port: HostnamePort,
        options: TcpOutletOptions,
    ) -> Result<()> {
        let access_control = options.incoming_access_control.clone();

        options.setup_flow_control_for_outlet_listener(ctx.flow_controls(), &address);

        let worker = Self::new(registry, hostname_port, options);
        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control_arc(access_control)
            .with_outgoing_access_control(DenyAll)
            .start(ctx)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpOutletListenWorker {
    type Context = Context;
    type Message = NeutralMessage;

    #[instrument(skip_all, name = "TcpOutletListenWorker::initialize")]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.add_outlet_listener_worker(&ctx.address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::shutdown")]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_outlet_listener_worker(&ctx.address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::handle_message")]
    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let their_identifier = SecureChannelLocalInfo::find_info(msg.local_message())
            .map(|l| l.their_identifier())
            .ok();
        let return_route = msg.return_route();
        let src_addr = msg.src_addr();
        let body = msg.into_body()?.into_vec();
        let msg = PortalMessage::decode(&body)?;

        if !matches!(msg, PortalMessage::Ping) {
            return Err(TransportError::Protocol)?;
        }

        let addresses = Addresses::generate(PortalType::Outlet);

        TcpOutletOptions::setup_flow_control_for_outlet(ctx.flow_controls(), &addresses, &src_addr);

        TcpPortalWorker::start_new_outlet(
            ctx,
            self.registry.clone(),
            self.hostname_port.clone(),
            self.options.tls,
            return_route.clone(),
            their_identifier,
            addresses.clone(),
            self.options.incoming_access_control.clone(),
            self.options.outgoing_access_control.clone(),
        )
        .await?;

        debug!("Created Tcp Outlet at {}", addresses.sender_remote);

        Ok(())
    }
}
