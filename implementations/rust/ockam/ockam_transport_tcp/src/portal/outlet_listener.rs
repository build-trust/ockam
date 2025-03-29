use crate::portal::addresses::{Addresses, PortalType};
use crate::portal::outlet_listener_registry::{MapKey, OutletListenerRegistry};
use crate::{portal::TcpPortalWorker, PortalMessage, TcpOutletOptions, TcpRegistry};
use ockam_core::{
    async_trait, route, Address, AllowAll, LocalMessage, NeutralMessage, Result, Routed,
    SecureChannelLocalInfo, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::{HostnamePort, TransportError};
use tracing::{debug, instrument, Level};

/// A TCP Portal Outlet listen worker
///
/// TCP Portal Outlet listen workers are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_outlet`](crate::TcpTransport::create_outlet).
pub(crate) struct TcpOutletListenWorker {
    registry: TcpRegistry,
    hostname_port: HostnamePort,
    options: TcpOutletOptions,
    outlet_registry: OutletListenerRegistry,
}

impl TcpOutletListenWorker {
    /// Create a new `TcpOutletListenWorker`
    fn new(registry: TcpRegistry, hostname_port: HostnamePort, options: TcpOutletOptions) -> Self {
        Self {
            registry,
            hostname_port,
            options,
            outlet_registry: Default::default(),
        }
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::start", level = Level::TRACE)]
    pub(crate) fn start(
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
            .start(ctx)?;

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
            debug!(
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

    #[instrument(skip_all, name = "TcpOutletListenWorker::initialize", level = Level::TRACE)]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .add_outlet_listener_worker(ctx.primary_address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::shutdown", level = Level::TRACE)]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_outlet_listener_worker(ctx.primary_address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpOutletListenWorker::handle_message", level = Level::TRACE)]
    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let their_identifier = SecureChannelLocalInfo::find_info(msg.local_message())
            .map(|l| l.their_identifier())
            .ok();

        let src_addr = msg.src_addr().clone();
        let msg = msg.into_local_message();

        let remote_address = msg.return_route.recipient()?.clone();

        let map_key = MapKey {
            identifier: their_identifier.clone(),
            remote_address,
        };

        if self.options.skip_handshake {
            let sender_remote = self
                .outlet_registry
                .started_workers
                .read()
                .unwrap()
                .get(&map_key)
                .cloned();

            if let Some(sender_remote) = sender_remote {
                return Self::reroute_msg(ctx, sender_remote, msg).await;
            }
        } else {
            let msg = PortalMessage::decode(msg.payload())?;

            if !matches!(msg, PortalMessage::Ping) {
                return Err(TransportError::Protocol)?;
            }
        }

        let addresses = Addresses::generate(PortalType::Outlet);

        if self.options.skip_handshake {
            TcpPortalWorker::start_new_outlet_no_handshake(
                ctx,
                self.registry.clone(),
                self.hostname_port.clone(),
                self.options.tls,
                msg.return_route.clone(),
                their_identifier,
                addresses.clone(),
                self.options.outgoing_access_control.clone(),
                self.options.portal_payload_length,
                map_key.clone(),
                self.outlet_registry.clone(),
                self.options.enable_mptcp,
            )?;

            debug!("Created Tcp Outlet at {}", addresses.sender_remote);

            self.outlet_registry
                .started_workers
                .write()
                .unwrap()
                .insert(map_key.clone(), addresses.sender_remote.clone());

            Self::reroute_msg(ctx, addresses.sender_remote, msg).await?;
        } else {
            TcpOutletOptions::setup_flow_control_for_outlet(
                ctx.flow_controls(),
                &addresses,
                &src_addr,
            );

            TcpPortalWorker::start_new_outlet(
                ctx,
                self.registry.clone(),
                self.hostname_port.clone(),
                self.options.tls,
                msg.return_route.clone(),
                their_identifier,
                addresses.clone(),
                self.options.incoming_access_control.clone(),
                self.options.outgoing_access_control.clone(),
                self.options.portal_payload_length,
                self.options.enable_mptcp,
            )?;

            debug!("Created Tcp Outlet at {}", addresses.sender_remote);
        }

        Ok(())
    }
}
