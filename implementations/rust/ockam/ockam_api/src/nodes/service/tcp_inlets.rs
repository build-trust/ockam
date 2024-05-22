use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

use tokio::time::timeout;

use crate::address::get_free_address_for;
use crate::DefaultAddress;
use ockam::identity::Identifier;
use ockam::tcp::TcpInletOptions;
use ockam::udp::{UdpPunctureNegotiation, UdpTransport};
use ockam::Result;
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    async_trait, route, AsyncTryClone, IncomingAccessControl, OutgoingAccessControl, Route,
};
use ockam_multiaddr::proto::Project as ProjectProto;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;
use ockam_transport_tcp::TcpInlet;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::registry::InletInfo;
use crate::nodes::{BackgroundNodeClient, InMemoryNode};
use crate::session::sessions::{
    ConnectionStatus, CurrentInletStatus, ReplacerOutcome, ReplacerOutputKind, Session,
    SessionReplacer, MAX_CONNECT_TIME, MAX_RECOVERY_TIME,
};
use crate::session::MedicHandle;

use super::{NodeManager, NodeManagerWorker, SecureChannelType};

impl NodeManagerWorker {
    pub(super) async fn get_inlets(&self) -> Result<Response<Vec<InletStatus>>, Response<Error>> {
        let inlets = self.node_manager.list_inlets().await;
        Ok(Response::ok().body(inlets))
    }

    #[instrument(skip_all)]
    pub(super) async fn create_inlet(
        &self,
        ctx: &Context,
        create_inlet: CreateInlet,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        let CreateInlet {
            listen_addr,
            outlet_addr,
            alias,
            authorized,
            prefix_route,
            suffix_route,
            wait_for_outlet_duration,
            policy_expression,
            wait_connection,
            secure_channel_identifier,
            enable_udp_puncture,
            disable_tcp_fallback,
        } = create_inlet;
        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                prefix_route,
                suffix_route,
                outlet_addr,
                alias,
                policy_expression,
                wait_for_outlet_duration,
                authorized,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
            )
            .await
        {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn delete_inlet(
        &self,
        alias: &str,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        match self.node_manager.delete_inlet(alias).await {
            Ok(status) => Ok(Response::ok().body(status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn show_inlet(
        &self,
        alias: &str,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        match self.node_manager.show_inlet(alias).await {
            Some(inlet) => Ok(Response::ok().body(inlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Inlet with alias {alias} not found"
            ))),
        }
    }
}

impl NodeManager {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn create_inlet(
        self: &Arc<Self>,
        ctx: &Context,
        listen_addr: String,
        prefix_route: Route,
        suffix_route: Route,
        outlet_addr: MultiAddr,
        alias: String,
        policy_expression: Option<PolicyExpression>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
        secure_channel_identifier: Option<Identifier>,
        enable_udp_puncture: bool,
        // TODO: Introduce mode enum
        disable_tcp_fallback: bool,
    ) -> Result<InletStatus> {
        info!("Handling request to create inlet portal");
        debug! {
            listen_addr = %listen_addr,
            prefix = %prefix_route,
            suffix = %suffix_route,
            outlet_addr = %outlet_addr,
            %alias,
            %enable_udp_puncture,
            %disable_tcp_fallback,
            "Creating inlet portal"
        }

        let udp_transport = if enable_udp_puncture {
            Some(self.udp_transport.clone().ok_or(ockam_core::Error::new(
                Origin::Transport,
                Kind::Invalid,
                "Can't enable UDP puncture or non UDP node",
            ))?)
        } else {
            None
        };

        // the port could be zero, to simplify the following code we
        // resolve the address to a full socket address
        let socket_addr = SocketAddr::from_str(&listen_addr)
            .map_err(|err| ockam_core::Error::new(Origin::Transport, Kind::Invalid, err))?;
        let listen_addr = if listen_addr.ends_with(":0") {
            get_free_address_for(&socket_addr.ip().to_string())
                .map_err(|err| ockam_core::Error::new(Origin::Transport, Kind::Invalid, err))?
        } else {
            socket_addr
        };

        // Check registry for duplicated alias or bind address
        {
            let registry = &self.registry.inlets;

            // Check that there is no entry in the registry with the same alias
            if registry.contains_key(&alias).await {
                let message = format!("A TCP inlet with alias '{alias}' already exists");
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::AlreadyExists,
                    message,
                ));
            }

            // Check that there is no entry in the registry with the same TCP bind address
            if registry
                .values()
                .await
                .iter()
                .any(|inlet| inlet.bind_addr == listen_addr.to_string())
            {
                let message =
                    format!("A TCP inlet with bind tcp address '{listen_addr}' already exists");
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::AlreadyExists,
                    message,
                ));
            }
        }

        let replacer = InletSessionReplacer {
            node_manager: self.clone(),
            udp_transport,
            context: Arc::new(ctx.async_try_clone().await?),
            listen_addr: listen_addr.to_string(),
            outlet_addr: outlet_addr.clone(),
            prefix_route,
            suffix_route,
            authorized,
            wait_for_outlet_duration: wait_for_outlet_duration.unwrap_or(MAX_CONNECT_TIME),
            resource: Resource::new(alias.clone(), ResourceType::TcpInlet),
            policy_expression,
            secure_channel_identifier,
            disable_tcp_fallback,
            connection: None,
            inlet: None,
            handle: None,
        };

        let _ = self
            .cli_state
            .create_tcp_inlet(&self.node_name, &listen_addr, &outlet_addr, &alias)
            .await?;

        let mut session = Session::new(replacer);
        let outcome = if wait_connection {
            let result =
                MedicHandle::connect(&mut session)
                    .await
                    .map(|outcome| match outcome.kind {
                        ReplacerOutputKind::Inlet(status) => status,
                        _ => {
                            panic!("Unexpected outcome: {:?}", outcome)
                        }
                    });

            match result {
                Ok(status) => Some(status),
                Err(err) => {
                    warn!("Failed to create inlet: {err}");
                    None
                }
            }
        } else {
            None
        };

        self.registry
            .inlets
            .insert(
                alias.clone(),
                InletInfo::new(&listen_addr.to_string(), outlet_addr.clone(), session),
            )
            .await;

        let tcp_inlet_status = InletStatus::new(
            listen_addr.to_string(),
            outcome.clone().map(|s| s.worker.address().to_string()),
            &alias,
            None,
            outcome.clone().map(|s| s.route.to_string()),
            outcome
                .as_ref()
                .map(|s| s.connection_status)
                .unwrap_or(ConnectionStatus::Down),
            outlet_addr.to_string(),
        );

        Ok(tcp_inlet_status)
    }

    pub async fn delete_inlet(&self, alias: &str) -> Result<InletStatus> {
        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = self.registry.inlets.remove(alias).await {
            debug!(%alias, "Successfully removed inlet from node registry");
            inlet_to_delete.session.close().await?;
            self.resources().delete_resource(&alias.into()).await?;
            self.cli_state
                .delete_tcp_inlet(&self.node_name, alias)
                .await?;
            Ok(InletStatus::new(
                inlet_to_delete.bind_addr,
                None,
                alias,
                None,
                None,
                ConnectionStatus::Down,
                inlet_to_delete.outlet_addr.to_string(),
            ))
        } else {
            error!(%alias, "Inlet not found in the node registry");
            let message = format!("Inlet with alias {alias} not found");
            Err(ockam_core::Error::new(
                Origin::Node,
                Kind::NotFound,
                message,
            ))
        }
    }

    pub async fn show_inlet(&self, alias: &str) -> Option<InletStatus> {
        info!(%alias, "Handling request to show inlet portal");
        if let Some(inlet_info) = self.registry.inlets.get(alias).await {
            if let Some(status) = inlet_info.session.status() {
                if let ReplacerOutputKind::Inlet(status) = &status.kind {
                    Some(InletStatus::new(
                        inlet_info.bind_addr.to_string(),
                        status.worker.address().to_string(),
                        alias,
                        None,
                        status.route.to_string(),
                        status.connection_status,
                        inlet_info.outlet_addr.to_string(),
                    ))
                } else {
                    panic!("Unexpected outcome: {:?}", status.kind)
                }
            } else {
                Some(InletStatus::new(
                    inlet_info.bind_addr.to_string(),
                    None,
                    alias,
                    None,
                    None,
                    ConnectionStatus::Down,
                    inlet_info.outlet_addr.to_string(),
                ))
            }
        } else {
            error!(%alias, "Inlet not found in the node registry");
            None
        }
    }

    pub async fn list_inlets(&self) -> Vec<InletStatus> {
        self.registry
            .inlets
            .entries()
            .await
            .iter()
            .map(|(alias, info)| {
                if let Some(status) = info.session.status().as_ref() {
                    match &status.kind {
                        ReplacerOutputKind::Inlet(status) => InletStatus::new(
                            &info.bind_addr,
                            status.worker.address().to_string(),
                            alias,
                            None,
                            status.route.to_string(),
                            status.connection_status,
                            info.outlet_addr.to_string(),
                        ),
                        _ => {
                            panic!("Unexpected outcome: {:?}", status.kind)
                        }
                    }
                } else {
                    InletStatus::new(
                        &info.bind_addr,
                        None,
                        alias,
                        None,
                        None,
                        ConnectionStatus::Down,
                        info.outlet_addr.to_string(),
                    )
                }
            })
            .collect()
    }
}

impl InMemoryNode {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: String,
        prefix_route: Route,
        suffix_route: Route,
        outlet_addr: MultiAddr,
        alias: String,
        policy_expression: Option<PolicyExpression>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
        secure_channel_identifier: Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
    ) -> Result<InletStatus> {
        self.node_manager
            .create_inlet(
                ctx,
                listen_addr.clone(),
                prefix_route.clone(),
                suffix_route.clone(),
                outlet_addr.clone(),
                alias,
                policy_expression,
                wait_for_outlet_duration,
                authorized,
                wait_connection,
                secure_channel_identifier,
                enable_udp_puncture,
                disable_tcp_fallback,
            )
            .await
    }
}

struct InletSessionReplacer {
    node_manager: Arc<NodeManager>,
    udp_transport: Option<UdpTransport>,
    context: Arc<Context>,
    listen_addr: String,
    outlet_addr: MultiAddr,
    prefix_route: Route,
    suffix_route: Route,
    authorized: Option<Identifier>,
    wait_for_outlet_duration: Duration,
    resource: Resource,
    policy_expression: Option<PolicyExpression>,
    secure_channel_identifier: Option<Identifier>,
    disable_tcp_fallback: bool,

    // current status
    connection: Option<Connection>,
    inlet: Option<Arc<TcpInlet>>,
    handle: Option<JoinHandle<()>>,
}

impl InletSessionReplacer {
    fn enable_udp_puncture(&self) -> bool {
        self.udp_transport.is_some()
    }

    async fn access_control(
        &self,
    ) -> Result<(
        Arc<dyn IncomingAccessControl>,
        Arc<dyn OutgoingAccessControl>,
    )> {
        let authority = {
            if let Some(p) = self.outlet_addr.first() {
                if let Some(p) = p.cast::<ProjectProto>() {
                    if let Ok(p) = self
                        .node_manager
                        .cli_state
                        .projects()
                        .get_project_by_name(&p)
                        .await
                    {
                        Some(p.authority_identifier()?)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        .or(self.node_manager.project_authority());

        self.node_manager
            .access_control(
                &self.context,
                authority,
                self.resource.clone(),
                Action::HandleMessage,
                self.policy_expression.clone(),
            )
            .await
    }

    async fn spawn_udp_puncture(
        &mut self,
        connection: &Connection,
        inlet: Arc<TcpInlet>, // TODO: PUNCTURE Replace with a RwLock
        disable_tcp_fallback: bool,
    ) -> Result<()> {
        let udp_transport = self.udp_transport.as_ref().ok_or(ockam_core::Error::new(
            Origin::Node,
            Kind::Invalid,
            "Couldn't create inlet with puncture",
        ))?;

        let mut main_route = connection.route()?;
        // FIXME: PUNCTURE trimming outlet part, but doesn't look good
        let main_route: Route = main_route.modify().pop_back().into();

        // FIXME: PUNCTURE don't assume listener address here
        let sc_side_route = route![main_route.clone(), DefaultAddress::SECURE_CHANNEL_LISTENER];

        // TODO: PUNCTURE make it a part of Replacer's state
        let sc_side = self
            .node_manager
            .create_secure_channel_internal(
                &self.context,
                sc_side_route,
                &self.node_manager.identifier(),
                self.authorized.clone().map(|authorized| vec![authorized]),
                None,
                // FIXME: PUNCTURE what is the right timeout here?
                Some(self.wait_for_outlet_duration),
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;

        let rendezvous_route = route![
            DefaultAddress::get_rendezvous_server_address(),
            DefaultAddress::RENDEZVOUS_SERVICE
        ];

        let (mut receiver, sender) = UdpPunctureNegotiation::start_negotiation(
            &self.context,
            route![
                main_route.clone(),
                DefaultAddress::UDP_PUNCTURE_NEGOTIATION_LISTENER
            ],
            udp_transport,
            rendezvous_route,
        )
        .await?;

        let mut receiver_clone = sender.subscribe();

        let handle = self.context.runtime().spawn(async move {
            let mut last_route = None;

            loop {
                match receiver.recv().await {
                    Ok(route) => {
                        if Some(&route) == last_route.as_ref() {
                            debug!("Route did not change, skipping");
                            continue;
                        }

                        // TODO: PUNCTURE add more meaningful return type from receiver
                        if !route.is_empty() {
                            info!("Updating route to UDP: {}", route);

                            // TODO: PUNCTURE monitor this side as well?
                            //  Also monitored on the UDP puncture level
                            // TODO: PUNCTURE handle error
                            let _res = sc_side.update_remote_node_route(route.clone());

                            let _res = inlet.unpause(route![sc_side.clone()]);
                        // TODO: PUNCTURE handle error
                        } else if disable_tcp_fallback {
                            error!("UDP puncture failed. TCP fallback is disabled.");
                            inlet.pause();
                        } else {
                            info!("Updating route to TCP");

                            let _res = inlet.unpause(route![main_route.clone()]);
                            // TODO: PUNCTURE handle error
                        }

                        last_route = Some(route);
                    }
                    // TODO: PUNCTURE handle more errors (overflow)
                    Err(err) => {
                        if disable_tcp_fallback {
                            // TODO: PUNCTURE improve logging

                            error!(
                                "Error waiting for the UDP puncture. TCP fallback is disabled. \
                                Error: {}",
                                err
                            );

                            inlet.pause();
                        } else {
                            info!("Error. Updating route to TCP");

                            let _res = inlet.unpause(route![main_route.clone()]);
                            // TODO: PUNCTURE handle error
                        }

                        break;
                    }
                }
            }
        });

        self.handle = Some(handle);

        // Wait for the completion
        if disable_tcp_fallback {
            match receiver_clone.recv().await {
                Ok(route) if !route.is_empty() => {}
                _ => {
                    return Err(ockam_core::Error::new(
                        Origin::Node,
                        Kind::Invalid,
                        "Couldn't create inlet with puncture",
                    ))?
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SessionReplacer for InletSessionReplacer {
    async fn create(&mut self) -> std::result::Result<ReplacerOutcome, ockam_core::Error> {
        // The addressing scheme is very flexible. Typically, the node connects to
        // the cloud via a secure channel and with another secure channel via
        // relay to the actual outlet on the target node. However, it is also
        // possible that there is just a single secure channel used to go directly
        // to another node.

        self.close().await;
        debug!(%self.outlet_addr, "creating new tcp inlet");

        let (incoming_ac, outgoing_ac) = self.access_control().await?;

        // The future that recreates the inlet:
        let future = async {
            let connection = self
                .node_manager
                .make_connection(
                    self.context.clone(),
                    &self.outlet_addr,
                    self.secure_channel_identifier
                        .clone()
                        .unwrap_or(self.node_manager.identifier()),
                    self.authorized.clone(),
                    Some(self.wait_for_outlet_duration),
                )
                .await?;

            let connection_route = connection.route()?;

            //we expect a fully normalized MultiAddr
            let normalized_route = route![
                self.prefix_route.clone(),
                connection_route,
                self.suffix_route.clone()
            ];
            let options = TcpInletOptions::new()
                .with_incoming_access_control(incoming_ac)
                .with_outgoing_access_control(outgoing_ac);

            let options = if self.enable_udp_puncture() && self.disable_tcp_fallback {
                options.paused()
            } else {
                options
            };

            // TODO: Instead just update the route in the existing inlet
            // Finally, attempt to create a new inlet using the new route:
            let inlet = self
                .node_manager
                .tcp_transport
                .create_inlet(self.listen_addr.clone(), normalized_route.clone(), options)
                .await?
                .clone();
            let inlet_address = inlet.processor_address().clone();
            let inlet = Arc::new(inlet);
            self.inlet = Some(inlet.clone());

            if self.enable_udp_puncture() {
                info!("Spawning UDP puncture future");
                self.spawn_udp_puncture(&connection, inlet, self.disable_tcp_fallback)
                    .await?;
                info!("Spawned UDP puncture future");
            }

            Ok(ReplacerOutcome {
                ping_route: connection.transport_route(),
                kind: ReplacerOutputKind::Inlet(CurrentInletStatus {
                    worker: inlet_address,
                    route: normalized_route,
                    connection_status: ConnectionStatus::Up,
                }),
            })
        };

        // The above future is given some limited time to succeed.
        match timeout(MAX_RECOVERY_TIME, future).await {
            Err(_) => {
                warn!(%self.outlet_addr, "timeout creating new tcp inlet");
                Err(ApiError::core("timeout"))
            }
            Ok(Err(e)) => {
                warn!(%self.outlet_addr, err = %e, "error creating new tcp inlet");
                Err(e)
            }
            Ok(Ok(route)) => Ok(route),
        }
    }

    async fn close(&mut self) {
        if let Some(connection) = self.connection.take() {
            let result = connection.close(&self.context, &self.node_manager).await;
            if let Err(err) = result {
                error!(?err, "Failed to close connection");
            }
        }

        if let Some(inlet) = self.inlet.take() {
            // The previous inlet worker needs to be stopped:
            let result = self
                .node_manager
                .tcp_transport
                .stop_inlet(inlet.processor_address().clone())
                .await;

            if let Err(err) = result {
                error!(
                    ?err,
                    "Failed to remove inlet with address {}",
                    inlet.processor_address()
                );
            }
        }
    }
}

#[async_trait]
pub trait Inlets {
    #[allow(clippy::too_many_arguments)]
    async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: &str,
        outlet_addr: &MultiAddr,
        alias: &str,
        authorized_identifier: &Option<Identifier>,
        policy_expression: &Option<PolicyExpression>,
        wait_for_outlet_timeout: Duration,
        wait_connection: bool,
        secure_channel_identifier: &Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
    ) -> miette::Result<Reply<InletStatus>>;

    async fn show_inlet(&self, ctx: &Context, alias: &str) -> miette::Result<Reply<InletStatus>>;

    async fn delete_inlet(&self, ctx: &Context, inlet_alias: &str) -> miette::Result<Reply<()>>;
}

#[async_trait]
impl Inlets for BackgroundNodeClient {
    async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: &str,
        outlet_addr: &MultiAddr,
        alias: &str,
        authorized_identifier: &Option<Identifier>,
        policy_expression: &Option<PolicyExpression>,
        wait_for_outlet_timeout: Duration,
        wait_connection: bool,
        secure_channel_identifier: &Option<Identifier>,
        enable_udp_puncture: bool,
        disable_tcp_fallback: bool,
    ) -> miette::Result<Reply<InletStatus>> {
        let request = {
            let via_project = outlet_addr.matches(0, &[ProjectProto::CODE.into()]);
            let mut payload = if via_project {
                CreateInlet::via_project(
                    listen_addr.into(),
                    outlet_addr.clone(),
                    alias.into(),
                    route![],
                    route![],
                    wait_connection,
                    enable_udp_puncture,
                    disable_tcp_fallback,
                )
            } else {
                CreateInlet::to_node(
                    listen_addr.into(),
                    outlet_addr.clone(),
                    alias.into(),
                    route![],
                    route![],
                    authorized_identifier.clone(),
                    wait_connection,
                    enable_udp_puncture,
                    disable_tcp_fallback,
                )
            };
            if let Some(e) = policy_expression.as_ref() {
                payload.set_policy_expression(e.clone())
            }
            if let Some(identifier) = secure_channel_identifier {
                payload.set_secure_channel_identifier(identifier.clone())
            }
            payload.set_wait_ms(wait_for_outlet_timeout.as_millis() as u64);
            Request::post("/node/inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }

    async fn show_inlet(&self, ctx: &Context, alias: &str) -> miette::Result<Reply<InletStatus>> {
        let request = Request::get(format!("/node/inlet/{alias}"));
        self.ask_and_get_reply(ctx, request).await
    }

    async fn delete_inlet(&self, ctx: &Context, inlet_alias: &str) -> miette::Result<Reply<()>> {
        let request = Request::delete(format!("/node/inlet/{inlet_alias}"));
        self.tell_and_get_reply(ctx, request).await
    }
}
