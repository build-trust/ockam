use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use crate::address::get_free_address_for;
use ockam::identity::Identifier;
use ockam::{Address, Result};
use ockam_abac::{Expr, Resource};
use ockam_core::api::{Error, Reply, Request, RequestHeader, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, AsyncTryClone, IncomingAccessControl, Route};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;
use ockam_transport_tcp::{TcpInletOptions, TcpOutletOptions};

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletList, InletStatus, OutletList, OutletStatus,
};
use crate::nodes::registry::{InletInfo, OutletInfo};
use crate::nodes::service::actions;
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
    pub(super) async fn get_inlets(&self) -> Result<Response<InletList>, Response<Error>> {
        let inlets = self.node_manager.list_inlets().await;
        Ok(Response::ok().body(inlets))
    }

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
        } = create_inlet;
        match self
            .node_manager
            .create_inlet(
                ctx,
                listen_addr,
                alias,
                prefix_route,
                suffix_route,
                outlet_addr,
                policy_expression,
                wait_for_outlet_duration,
                authorized,
                wait_connection,
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
    pub(super) async fn create_outlet(
        &self,
        ctx: &Context,
        create_outlet: CreateOutlet,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        let CreateOutlet {
            socket_addr,
            worker_addr,
            alias,
            reachable_from_default_secure_channel,
            policy_expression,
        } = create_outlet;

        match self
            .node_manager
            .create_outlet(
                ctx,
                socket_addr,
                worker_addr,
                alias,
                reachable_from_default_secure_channel,
                None,
                policy_expression,
            )
            .await
        {
            Ok(outlet_status) => Ok(Response::ok().body(outlet_status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn delete_outlet(
        &self,
        alias: &str,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.delete_outlet(alias).await {
            Ok(res) => match res {
                Some(outlet_info) => Ok(Response::ok().body(OutletStatus::new(
                    outlet_info.socket_addr,
                    outlet_info.worker_addr.clone(),
                    alias,
                    None,
                ))),
                None => Err(Response::bad_request_no_request(&format!(
                    "Outlet with alias {alias} not found"
                ))),
            },
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn show_outlet(
        &self,
        alias: &str,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.show_outlet(alias).await {
            Some(outlet) => Ok(Response::ok().body(outlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Outlet with alias {alias} not found"
            ))),
        }
    }

    pub(super) async fn get_outlets(&self, req: &RequestHeader) -> Response<OutletList> {
        Response::ok()
            .with_headers(req)
            .body(self.node_manager.list_outlets().await)
    }
}

/// OUTLETS
impl NodeManager {
    #[instrument(skip(self, ctx))]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_outlet(
        &self,
        ctx: &Context,
        socket_addr: SocketAddr,
        worker_addr: Address,
        alias: String,
        reachable_from_default_secure_channel: bool,
        access_control: Option<Arc<dyn IncomingAccessControl>>,
        policy_expression: Option<Expr>,
    ) -> Result<OutletStatus> {
        info!(
            "Handling request to create outlet portal at {:?} with worker {:?}",
            socket_addr, worker_addr
        );

        // Check registry for duplicated alias
        if self.registry.outlets.contains_key(&alias).await {
            let message = format!("A TCP outlet with alias '{alias}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let access_control = if let Some(access_control) = access_control {
            access_control
        } else {
            self.access_control(
                Resource::new(&alias),
                actions::HANDLE_MESSAGE,
                self.authority(),
                policy_expression,
            )
            .await?
        };

        let options = {
            let options = TcpOutletOptions::new().with_incoming_access_control(access_control);
            let options = if self.authority().is_none() {
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

        let res = self
            .tcp_transport
            .create_tcp_outlet(worker_addr.clone(), socket_addr, options)
            .await;

        Ok(match res {
            Ok(_) => {
                // TODO: Use better way to store outlets?
                self.registry
                    .outlets
                    .insert(
                        alias.clone(),
                        OutletInfo::new(&socket_addr, Some(&worker_addr)),
                    )
                    .await;

                OutletStatus::new(socket_addr, worker_addr, alias, None)
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

    pub async fn delete_outlet(&self, alias: &str) -> Result<Option<OutletInfo>> {
        info!(%alias, "Handling request to delete outlet portal");
        if let Some(deleted_outlet) = self.registry.outlets.remove(alias).await {
            debug!(%alias, "Successfully removed outlet from node registry");
            if let Err(e) = self
                .tcp_transport
                .stop_outlet(deleted_outlet.worker_addr.clone())
                .await
            {
                warn!(%alias, %e, "Failed to stop outlet worker");
            }
            trace!(%alias, "Successfully stopped outlet");
            Ok(Some(deleted_outlet))
        } else {
            warn!(%alias, "Outlet not found in the node registry");
            Ok(None)
        }
    }

    pub(super) async fn show_outlet(&self, alias: &str) -> Option<OutletStatus> {
        info!(%alias, "Handling request to show outlet portal");
        if let Some(outlet_to_show) = self.registry.outlets.get(alias).await {
            debug!(%alias, "Outlet not found in node registry");
            Some(OutletStatus::new(
                outlet_to_show.socket_addr,
                outlet_to_show.worker_addr.clone(),
                alias,
                None,
            ))
        } else {
            error!(%alias, "Outlet not found in the node registry");
            None
        }
    }
}

/// INLETS
impl NodeManager {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_inlet(
        self: &Arc<Self>,
        ctx: &Context,
        listen_addr: String,
        alias: String,
        prefix_route: Route,
        suffix_route: Route,
        outlet_addr: MultiAddr,
        policy_expression: Option<Expr>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
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
        let listen_addr = if listen_addr.ends_with(":0") {
            let socket_addr = SocketAddr::from_str(&listen_addr)
                .map_err(|err| ockam_core::Error::new(Origin::Transport, Kind::Invalid, err))?;
            get_free_address_for(&socket_addr.ip().to_string())
                .map_err(|err| ockam_core::Error::new(Origin::Transport, Kind::Invalid, err))?
                .to_string()
        } else {
            listen_addr
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
                .any(|inlet| inlet.bind_addr == listen_addr)
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
            listen_addr: listen_addr.clone(),
            addr: outlet_addr.clone(),
            prefix_route,
            suffix_route,
            authorized,
            resource: Resource::new(&alias),
            wait_for_outlet_duration: wait_for_outlet_duration.unwrap_or(MAX_CONNECT_TIME),
            policy_expression,
            connection: None,
            inlet_address: None,
        };

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
                InletInfo::new(&listen_addr, outlet_addr.clone(), session),
            )
            .await;

        Ok(InletStatus::new(
            listen_addr.clone(),
            outcome.as_ref().map(|s| s.worker.address().to_string()),
            alias.clone(),
            None,
            outcome.as_ref().map(|s| s.route.to_string()),
            outcome
                .as_ref()
                .map(|s| s.connection_status)
                .unwrap_or(ConnectionStatus::Down),
            outlet_addr.to_string(),
        ))
    }

    pub async fn delete_inlet(&self, alias: &str) -> Result<InletStatus> {
        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = self.registry.inlets.remove(alias).await {
            debug!(%alias, "Successfully removed inlet from node registry");
            inlet_to_delete.session.close().await?;
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

    pub async fn list_inlets(&self) -> InletList {
        InletList::new(
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
                .collect(),
        )
    }
}

impl InMemoryNode {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_inlet(
        &self,
        ctx: &Context,
        listen_addr: String,
        alias: String,
        prefix_route: Route,
        suffix_route: Route,
        outlet_addr: MultiAddr,
        policy_expression: Option<Expr>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
    ) -> Result<InletStatus> {
        self.node_manager
            .create_inlet(
                ctx,
                listen_addr.clone(),
                alias,
                prefix_route.clone(),
                suffix_route.clone(),
                outlet_addr.clone(),
                policy_expression,
                wait_for_outlet_duration,
                authorized,
                wait_connection,
            )
            .await
    }
}

struct InletSessionReplacer {
    node_manager: Arc<NodeManager>,
    context: Arc<Context>,
    listen_addr: String,
    addr: MultiAddr,
    prefix_route: Route,
    suffix_route: Route,
    authorized: Option<Identifier>,
    resource: Resource,
    wait_for_outlet_duration: Duration,
    policy_expression: Option<Expr>,

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
        debug!(%self.addr, "creating new tcp inlet");

        // create the access_control
        let access_control = {
            let authority = {
                if let Some(p) = self.addr.first() {
                    if let Some(p) = p.cast::<Project>() {
                        let projects = self
                            .node_manager
                            .cli_state
                            .get_projects_grouped_by_name()
                            .await?;
                        if let Some(p) = projects.get(&*p) {
                            Some(p.authority_identifier().await?)
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
            .or(self.node_manager.authority());

            self.node_manager
                .access_control(
                    self.resource.clone(),
                    actions::HANDLE_MESSAGE,
                    authority,
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
                    &self.addr,
                    self.node_manager.identifier(),
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
            let options = TcpInletOptions::new().with_incoming_access_control(access_control);

            // Finally, attempt to create a new inlet using the new route:
            let inlet_address = self
                .node_manager
                .tcp_transport
                .create_inlet(self.listen_addr.clone(), normalized_route.clone(), options)
                .await?
                .1;
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
                warn!(%self.addr, "timeout creating new tcp inlet");
                Err(ApiError::core("timeout"))
            }
            Ok(Err(e)) => {
                warn!(%self.addr, err = %e, "error creating new tcp inlet");
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
        policy_expression: &Option<Expr>,
        wait_for_outlet_timeout: Duration,
        validate: bool,
    ) -> miette::Result<Reply<InletStatus>>;

    async fn show_inlet(
        &self,
        ctx: &Context,
        inlet_alias: &str,
    ) -> miette::Result<Reply<InletStatus>>;

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
        policy_expression: &Option<Expr>,
        wait_for_outlet_timeout: Duration,
        wait_connection: bool,
    ) -> miette::Result<Reply<InletStatus>> {
        let request = {
            let via_project = outlet_addr.matches(0, &[Project::CODE.into()]);
            let mut payload = if via_project {
                CreateInlet::via_project(
                    listen_addr.to_string(),
                    outlet_addr.clone(),
                    alias,
                    route![],
                    route![],
                    wait_connection,
                )
            } else {
                CreateInlet::to_node(
                    listen_addr.to_string(),
                    outlet_addr.clone(),
                    alias,
                    route![],
                    route![],
                    authorized_identifier.clone(),
                    wait_connection,
                )
            };
            if let Some(e) = policy_expression.as_ref() {
                payload.set_policy_expression(e.clone())
            }
            payload.set_wait_ms(wait_for_outlet_timeout.as_millis() as u64);
            Request::post("/node/inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }

    async fn show_inlet(
        &self,
        ctx: &Context,
        inlet_alias: &str,
    ) -> miette::Result<Reply<InletStatus>> {
        let request = Request::get(format!("/node/inlet/{inlet_alias}"));
        self.ask_and_get_reply(ctx, request).await
    }

    async fn delete_inlet(&self, ctx: &Context, inlet_alias: &str) -> miette::Result<Reply<()>> {
        let request = Request::delete(format!("/node/inlet/{inlet_alias}"));
        self.tell_and_get_reply(ctx, request).await
    }
}
