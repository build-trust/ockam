use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::timeout;

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
    ConnectionStatus, Replacer, Session, MAX_CONNECT_TIME, MAX_RECOVERY_TIME,
};

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
                wait_for_outlet_duration,
                authorized,
                policy_expression,
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
        &self,
        connection: Connection,
        listen_addr: String,
        alias: String,
        prefix_route: Route,
        suffix_route: Route,
        outlet_addr: MultiAddr,
        policy_expression: Option<Expr>,
    ) -> Result<(InletStatus, Arc<dyn IncomingAccessControl>)> {
        info!("Handling request to create inlet portal");
        debug! {
            listen_addr = %listen_addr,
            prefix = %prefix_route,
            suffix = %suffix_route,
            outlet_addr = %outlet_addr,
            %alias,
            "Creating inlet portal"
        }

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

        let outlet_route = connection.route(self.tcp_transport()).await?;
        let outlet_route = route![prefix_route.clone(), outlet_route, suffix_route.clone()];

        let authority = {
            if let Some(p) = outlet_addr.first() {
                if let Some(p) = p.cast::<Project>() {
                    let projects = self.cli_state.get_projects_grouped_by_name().await?;
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
        .or(self.authority());

        let access_control = self
            .access_control(
                Resource::new(&alias),
                actions::HANDLE_MESSAGE,
                authority,
                policy_expression,
            )
            .await?;

        let options = TcpInletOptions::new().with_incoming_access_control(access_control.clone());
        let res = self
            .tcp_transport
            .create_inlet(listen_addr.clone(), outlet_route.clone(), options)
            .await;

        Ok(match res {
            Ok((socket_address, worker_addr)) => {
                //when using 0 port, the chosen port will be populated
                //in the returned socket address
                let listen_addr = socket_address.to_string();

                // TODO: Use better way to store inlets?
                self.registry
                    .inlets
                    .insert(
                        alias.clone(),
                        InletInfo::new(&listen_addr, Some(&worker_addr), &outlet_route),
                    )
                    .await;
                (
                    InletStatus::new(
                        listen_addr,
                        worker_addr.to_string(),
                        alias,
                        None,
                        outlet_route.to_string(),
                        ConnectionStatus::Up,
                    ),
                    access_control,
                )
            }
            Err(e) => {
                warn!(to = %outlet_addr, err = %e, "Failed to create TCP inlet");
                let message = format!("Failed to create TCP inlet: {}", e);
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Internal,
                    message,
                ));
            }
        })
    }

    pub async fn delete_inlet(&self, alias: &str) -> Result<InletStatus> {
        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = self.registry.inlets.remove(alias).await {
            debug!(%alias, "Successfully removed inlet from node registry");
            match self
                .tcp_transport
                .stop_inlet(inlet_to_delete.worker_addr.clone())
                .await
            {
                Ok(_) => {
                    debug!(%alias, "Successfully stopped inlet");
                    Ok(InletStatus::new(
                        inlet_to_delete.bind_addr,
                        inlet_to_delete.worker_addr.to_string(),
                        alias,
                        None,
                        inlet_to_delete.outlet_route.to_string(),
                        ConnectionStatus::Down,
                    ))
                }
                Err(e) => {
                    error!(%alias, "Failed to remove inlet from node registry");
                    let message = format!("Failed to remove inlet with alias {alias}. {}", e);
                    Err(ockam_core::Error::new(
                        Origin::Node,
                        Kind::Internal,
                        message,
                    ))
                }
            }
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
        if let Some(inlet_to_show) = self.registry.inlets.get(alias).await {
            let status = self
                .medic_handle
                .status_of(&format!("inlet-{alias}"))
                .unwrap_or(ConnectionStatus::Down);

            debug!(%alias, "Inlet not found in node registry");
            Some(InletStatus::new(
                inlet_to_show.bind_addr.to_string(),
                inlet_to_show.worker_addr.address(),
                alias,
                None,
                inlet_to_show.outlet_route.to_string(),
                status,
            ))
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
                    let status = self
                        .medic_handle
                        .status_of(&format!("inlet-{alias}"))
                        .unwrap_or(ConnectionStatus::Down);

                    InletStatus::new(
                        &info.bind_addr,
                        info.worker_addr.to_string(),
                        alias,
                        None,
                        info.outlet_route.to_string(),
                        status,
                    )
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
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        policy_expression: Option<Expr>,
    ) -> Result<InletStatus> {
        // The addressing scheme is very flexible. Typically the node connects to
        // the cloud via secure channel and the with another secure channel via
        // relay to the actual outlet on the target node. However it is also
        // possible that there is just a single secure channel used to go directly
        // to another node.
        let duration = wait_for_outlet_duration.unwrap_or(Duration::from_secs(5));
        let connection_ctx = Arc::new(ctx.async_try_clone().await?);
        let connection = self
            .make_connection(
                connection_ctx.clone(),
                &outlet_addr,
                self.identifier(),
                authorized.clone(),
                Some(duration),
            )
            .await?;

        let (inlet, access_control) = self
            .node_manager
            .create_inlet(
                connection.clone(),
                listen_addr.clone(),
                alias,
                prefix_route.clone(),
                suffix_route.clone(),
                outlet_addr.clone(),
                policy_expression,
            )
            .await?;
        if !connection.route(self.tcp_transport()).await?.is_empty() {
            debug! {
                %inlet.alias,
                %inlet.bind_addr,
                %inlet.worker_addr,
                ping_addr = %connection.transport_route(),
                "Creating session for TCP inlet"
            };
            let mut session = Session::new(
                connection.transport_route(),
                format!("inlet-{}", inlet.alias),
            );

            let repl = Self::portal_replacer(
                self.node_manager.clone(),
                connection_ctx,
                connection,
                Address::from_string(inlet.worker_addr.clone()),
                listen_addr,
                outlet_addr,
                prefix_route,
                suffix_route,
                authorized,
                access_control,
            );
            session.set_replacer(repl);
            self.add_session(session);
        };
        Ok(inlet)
    }

    /// Create a session replacer.
    ///
    /// This returns a function that accepts the previous ping address (e.g.
    /// the secure channel worker address) and constructs the whole route
    /// again.
    #[allow(clippy::too_many_arguments)]
    fn portal_replacer(
        node_manager: Arc<NodeManager>,
        ctx: Arc<Context>,
        connection: Connection,
        inlet_address: Address,
        bind: String,
        addr: MultiAddr,
        prefix_route: Route,
        suffix_route: Route,
        authorized: Option<Identifier>,
        access: Arc<dyn IncomingAccessControl>,
    ) -> Replacer {
        let connection_arc = Arc::new(Mutex::new(connection.clone()));
        let inlet_address_arc = Arc::new(Mutex::new(inlet_address));
        let node_manager = node_manager.clone();

        Box::new(move |previous_addr| {
            let addr = addr.clone();
            let authorized = authorized.clone();
            let bind = bind.clone();
            let access = access.clone();
            let ctx = ctx.clone();
            let connection_arc = connection_arc.clone();
            let inlet_address_arc = inlet_address_arc.clone();
            let inlet_address = inlet_address_arc.lock().unwrap().clone();
            let prefix_route = prefix_route.clone();
            let suffix_route = suffix_route.clone();
            let previous_connection = connection_arc.lock().unwrap().clone();
            let node_manager = node_manager.clone();
            Box::pin(async move {
                debug!(%previous_addr, %addr, "creating new tcp inlet");
                // The future that recreates the inlet:
                let f = async {
                    //stop/delete previous secure channels
                    for encryptor in &previous_connection.secure_channel_encryptors {
                        let result = node_manager.delete_secure_channel(&ctx, encryptor).await;
                        if let Err(error) = result {
                            //we can't do much more
                            debug!("cannot delete secure channel `{encryptor}`: {error}");
                        }
                    }
                    if let Some(tcp_connection) = previous_connection.tcp_connection.as_ref() {
                        if let Err(error) = node_manager
                            .tcp_transport
                            .disconnect(tcp_connection.sender_address().clone())
                            .await
                        {
                            debug!("cannot stop tcp worker `{tcp_connection}`: {error}");
                        }
                    }

                    // The previous inlet worker needs to be stopped:
                    if let Err(error) = node_manager
                        .tcp_transport
                        .stop_inlet(inlet_address.clone())
                        .await
                    {
                        debug!("cannot stop inlet `{inlet_address}`: {error}");
                    }

                    // Now a connection attempt is made
                    let new_connection = node_manager
                        .make_connection(
                            ctx.clone(),
                            &addr,
                            node_manager.identifier(),
                            authorized,
                            Some(MAX_CONNECT_TIME),
                        )
                        .await?;
                    *connection_arc.lock().unwrap() = new_connection.clone();
                    let connection_route =
                        new_connection.route(node_manager.tcp_transport()).await?;

                    //we expect a fully normalized MultiAddr
                    let normalized_route = route![prefix_route, connection_route, suffix_route];
                    let options = TcpInletOptions::new().with_incoming_access_control(access);

                    // Finally attempt to create a new inlet using the new route:
                    let new_inlet_address = node_manager
                        .tcp_transport
                        .create_inlet(bind, normalized_route, options)
                        .await?
                        .1;
                    *inlet_address_arc.lock().unwrap() = new_inlet_address;

                    Ok(new_connection.transport_route())
                };

                // The above future is given some limited time to succeed.
                match timeout(MAX_RECOVERY_TIME, f).await {
                    Err(_) => {
                        warn!(%addr, "timeout creating new tcp inlet");
                        Err(ApiError::core("timeout"))
                    }
                    Ok(Err(e)) => {
                        warn!(%addr, err = %e, "error creating new tcp inlet");
                        Err(e)
                    }
                    Ok(Ok(route)) => Ok(route),
                }
            })
        })
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
                )
            } else {
                CreateInlet::to_node(
                    listen_addr.to_string(),
                    outlet_addr.clone(),
                    alias,
                    route![],
                    route![],
                    authorized_identifier.clone(),
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
