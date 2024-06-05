use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use crate::address::get_free_address_for;
use ockam::identity::Identifier;
use ockam::tcp::{TcpInletOptions, TcpOutletOptions};
use ockam::transport::HostnamePort;
use ockam::{Address, Result};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Reply, Request, RequestHeader, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, AsyncTryClone, Route};
use ockam_multiaddr::proto::Project as ProjectProto;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletStatus, OutletAccessControl, OutletStatus,
};
use crate::nodes::registry::{InletInfo, OutletInfo};
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::{BackgroundNodeClient, InMemoryNode};
use crate::session::sessions::{
    ConnectionStatus, CurrentInletStatus, ReplacerOutcome, ReplacerOutputKind, Session,
    SessionReplacer, MAX_CONNECT_TIME, MAX_RECOVERY_TIME,
};
use crate::session::MedicHandle;

use super::{NodeManager, NodeManagerWorker};

/// INLETS
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

/// OUTLETS
impl NodeManagerWorker {
    #[instrument(skip_all)]
    pub(super) async fn create_outlet(
        &self,
        ctx: &Context,
        create_outlet: CreateOutlet,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        let CreateOutlet {
            hostname_port,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression,
            tls,
        } = create_outlet;

        match self
            .node_manager
            .create_outlet(
                ctx,
                hostname_port,
                tls,
                worker_addr,
                reachable_from_default_secure_channel,
                OutletAccessControl::WithPolicyExpression(policy_expression),
            )
            .await
        {
            Ok(outlet_status) => Ok(Response::ok().body(outlet_status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn delete_outlet(
        &self,
        worker_addr: &Address,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.delete_outlet(worker_addr).await {
            Ok(res) => match res {
                Some(outlet_info) => Ok(Response::ok().body(OutletStatus::new(
                    outlet_info.socket_addr,
                    outlet_info.worker_addr.clone(),
                    None,
                ))),
                None => Err(Response::bad_request_no_request(&format!(
                    "Outlet with address {worker_addr} not found"
                ))),
            },
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn show_outlet(
        &self,
        worker_addr: &Address,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.show_outlet(worker_addr).await {
            Some(outlet) => Ok(Response::ok().body(outlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Outlet with address {worker_addr} not found"
            ))),
        }
    }

    pub(super) async fn get_outlets(&self, req: &RequestHeader) -> Response<Vec<OutletStatus>> {
        Response::ok()
            .with_headers(req)
            .body(self.node_manager.list_outlets().await)
    }
}

/// OUTLETS
impl NodeManager {
    #[instrument(skip(self, ctx))]
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn create_outlet(
        &self,
        ctx: &Context,
        hostname_port: HostnamePort,
        tls: bool,
        worker_addr: Option<Address>,
        reachable_from_default_secure_channel: bool,
        access_control: OutletAccessControl,
    ) -> Result<OutletStatus> {
        let worker_addr = self
            .registry
            .outlets
            .generate_worker_addr(worker_addr)
            .await;

        info!(
            "Handling request to create outlet portal at {}:{} with worker {:?}",
            &hostname_port.hostname(),
            hostname_port.port(),
            worker_addr
        );

        // Check registry for a duplicated key
        if self.registry.outlets.contains_key(&worker_addr).await {
            let message = format!("A TCP outlet with address '{worker_addr}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let (incoming_ac, outgoing_ac) = match access_control {
            OutletAccessControl::AccessControl((incoming_ac, outgoing_ac)) => {
                (incoming_ac, outgoing_ac)
            }
            OutletAccessControl::WithPolicyExpression(expression) => {
                self.access_control(
                    ctx,
                    self.project_authority(),
                    Resource::new(worker_addr.address(), ResourceType::TcpOutlet),
                    Action::HandleMessage,
                    expression,
                )
                .await?
            }
        };

        let options = {
            let options = TcpOutletOptions::new()
                .with_incoming_access_control(incoming_ac)
                .with_outgoing_access_control(outgoing_ac)
                .with_tls(tls);
            let options = if self.project_authority().is_none() {
                options.as_consumer(&self.api_transport_flow_control_id)
            } else {
                options
            };
            if reachable_from_default_secure_channel {
                // Accept messages from the default secure channel listener
                if let Some(flow_control_id) = ctx
                    .flow_controls()
                    .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
                {
                    options.as_consumer(&flow_control_id)
                } else {
                    options
                }
            } else {
                options
            }
        };

        let socket_addr = hostname_port.to_socket_addr()?;
        let res = self
            .tcp_transport
            .create_tcp_outlet(worker_addr.clone(), hostname_port, options)
            .await;

        Ok(match res {
            Ok(_) => {
                // TODO: Use better way to store outlets?
                self.registry
                    .outlets
                    .insert(
                        worker_addr.clone(),
                        OutletInfo::new(&socket_addr, Some(&worker_addr)),
                    )
                    .await;

                self.cli_state
                    .create_tcp_outlet(&self.node_name, &socket_addr, &worker_addr, &None)
                    .await?
            }
            Err(e) => {
                warn!(at = %socket_addr, err = %e, "Failed to create TCP outlet");
                let message = format!("Failed to create outlet: {}", e);
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Internal,
                    message,
                ));
            }
        })
    }

    pub async fn delete_outlet(&self, worker_addr: &Address) -> Result<Option<OutletInfo>> {
        info!(%worker_addr, "Handling request to delete outlet portal");
        if let Some(deleted_outlet) = self.registry.outlets.remove(worker_addr).await {
            debug!(%worker_addr, "Successfully removed outlet from node registry");

            self.cli_state
                .delete_tcp_outlet(&self.node_name, worker_addr)
                .await?;
            self.resources()
                .delete_resource(&worker_addr.address().into())
                .await?;

            if let Err(e) = self
                .tcp_transport
                .stop_outlet(deleted_outlet.worker_addr.clone())
                .await
            {
                warn!(%worker_addr, %e, "Failed to stop outlet worker");
            }
            trace!(%worker_addr, "Successfully stopped outlet");
            Ok(Some(deleted_outlet))
        } else {
            warn!(%worker_addr, "Outlet not found in the node registry");
            Ok(None)
        }
    }

    pub(super) async fn show_outlet(&self, worker_addr: &Address) -> Option<OutletStatus> {
        info!(%worker_addr, "Handling request to show outlet portal");
        if let Some(outlet_to_show) = self.registry.outlets.get(worker_addr).await {
            debug!(%worker_addr, "Outlet not found in node registry");
            Some(OutletStatus::new(
                outlet_to_show.socket_addr,
                outlet_to_show.worker_addr.clone(),
                None,
            ))
        } else {
            error!(%worker_addr, "Outlet not found in the node registry");
            None
        }
    }
}

/// INLETS
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
    ) -> Result<InletStatus> {
        info!("Handling request to create inlet portal");
        debug! {
            listen_addr = %listen_addr,
            prefix = %prefix_route,
            suffix = %suffix_route,
            outlet_addr = %outlet_addr,
            %alias,
            "Creating inlet portal"
        }

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
            connection: None,
            inlet_address: None,
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
            )
            .await
    }
}

struct InletSessionReplacer {
    node_manager: Arc<NodeManager>,
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

    // current status
    connection: Option<Connection>,
    inlet_address: Option<Address>,
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

        // create the access_control
        let (incoming_ac, outgoing_ac) = {
            let authority = {
                if let Some(p) = self.outlet_addr.first() {
                    if let Some(p) = p.cast::<ProjectProto>() {
                        let projects = self
                            .node_manager
                            .cli_state
                            .projects()
                            .get_projects_grouped_by_name()
                            .await?;
                        if let Some(p) = projects.get(&*p) {
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
                .await?
        };

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

            // TODO: Instead just update the route in the existing inlet
            // Finally, attempt to create a new inlet using the new route:
            let inlet_address = self
                .node_manager
                .tcp_transport
                .create_inlet(self.listen_addr.clone(), normalized_route.clone(), options)
                .await?
                .processor_address()
                .clone();
            self.inlet_address = Some(inlet_address.clone());

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

        if let Some(inlet_address) = self.inlet_address.take() {
            // The previous inlet worker needs to be stopped:
            let result = self
                .node_manager
                .tcp_transport
                .stop_inlet(inlet_address.clone())
                .await;

            if let Err(err) = result {
                error!(?err, "Failed to remove inlet with address {inlet_address}");
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

#[async_trait]
pub trait Outlets {
    async fn create_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<OutletStatus>;
}

#[async_trait]
impl Outlets for BackgroundNodeClient {
    #[instrument(skip_all, fields(to = % to, from = ? from))]
    async fn create_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<OutletStatus> {
        let mut payload = CreateOutlet::new(to, tls, from.cloned(), true);
        if let Some(policy_expression) = policy_expression {
            payload.set_policy_expression(policy_expression);
        }
        let req = Request::post("/node/outlet").body(payload);
        let result: OutletStatus = self.ask(ctx, req).await?;
        Ok(result)
    }
}
