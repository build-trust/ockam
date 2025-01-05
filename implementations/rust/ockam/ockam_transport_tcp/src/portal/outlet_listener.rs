use crate::portal::addresses::{Addresses, PortalType};
use crate::{portal::TcpPortalWorker, PortalMessage, TcpOutletOptions, TcpRegistry};
use log::warn;
use ockam_core::compat::collections::HashMap;
use ockam_core::{
    async_trait, route, Address, AllowAll, Encodable, LocalInfoIdentifier, LocalMessage,
    NeutralMessage, Result, Routed, SecureChannelLocalInfo, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::HostnamePort;
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

    started_workers: HashMap<MapKey, Address>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct MapKey {
    identifier: Option<LocalInfoIdentifier>,
    remote_address: Address,
}

impl TcpOutletListenWorker {
    /// Create a new `TcpOutletListenWorker`
    fn new(registry: TcpRegistry, hostname_port: HostnamePort, options: TcpOutletOptions) -> Self {
        Self {
            registry,
            hostname_port,
            options,
            started_workers: Default::default(),
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
            .with_outgoing_access_control(AllowAll)
            .start(ctx)
            .await?;

        Ok(())
    }

    async fn reroute_msg(ctx: &Context, sender_remote: Address, msg: LocalMessage) -> Result<()> {
        let res = ctx
            .forward_from_address(
                LocalMessage::new()
                    .with_onward_route(route![sender_remote.clone()])
                    .with_return_route(msg.return_route)
                    .with_local_info(msg.local_info)
                    .with_payload(msg.payload),
                ctx.primary_address().clone(),
            )
            .await;

        if res.is_err() {
            warn!(
                "Couldn't forward message from the outlet to {}",
                sender_remote
            )
        }

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpOutletListenWorker {
    type Context = Context;
    type Message = NeutralMessage;

    #[instrument(skip_all, name = "TcpOutletListenWorker::initialize")]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .add_outlet_listener_worker(ctx.primary_address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::shutdown")]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_outlet_listener_worker(ctx.primary_address());

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
        let msg = msg.into_local_message();

        let remote_address = msg.return_route.recipient()?.clone();

        let map_key = MapKey {
            identifier: their_identifier.clone(),
            remote_address,
        };

        if let Some(sender_remote) = self.started_workers.get(&map_key) {
            Self::reroute_msg(ctx, sender_remote.clone(), msg).await?;

            return Ok(());
        }

        let addresses = Addresses::generate(PortalType::Outlet);

        TcpPortalWorker::start_new_outlet(
            ctx,
            self.registry.clone(),
            self.hostname_port.clone(),
            self.options.tls.clone(),
            msg.return_route.clone(),
            their_identifier,
            addresses.clone(),
            self.options.outgoing_access_control.clone(),
        )
        .await?;

        debug!("Created Tcp Outlet at {}", addresses.sender_remote);

        self.started_workers
            .insert(map_key.clone(), addresses.sender_remote.clone());

        ctx.send(msg.return_route.clone(), PortalMessage::Pong.encode()?)
            .await?;

        Self::reroute_msg(ctx, addresses.sender_remote, msg).await?;

        Ok(())
    }
}
