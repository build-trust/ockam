use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;

use minicbor::Decoder;

use ockam::compat::tokio::time::timeout;
use ockam::identity::Identifier;
use ockam::{Address, AsyncTryClone, Result};
use ockam_abac::Resource;
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, IncomingAccessControl, Route};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::{TcpInletOptions, TcpOutletOptions};

use crate::cli_state::StateDirTrait;
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::connection::{Connection, ConnectionInstance};
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletList, InletStatus, OutletList, OutletStatus,
};
use crate::nodes::registry::{InletInfo, OutletInfo};
use crate::nodes::service::random_alias;
use crate::session::sessions::{Replacer, Session, MAX_CONNECT_TIME, MAX_RECOVERY_TIME};
use crate::{actions, resources, DefaultAddress};

use super::{NodeManager, NodeManagerWorker};

impl NodeManager {
    pub async fn create_outlet(
        &self,
        ctx: &Context,
        socket_addr: SocketAddr,
        worker_addr: Address,
        alias: Option<String>,
        reachable_from_default_secure_channel: bool,
    ) -> Result<OutletStatus> {
        info!(
            "Handling request to create outlet portal at {:?}",
            socket_addr
        );
        let resource = alias
            .as_deref()
            .map(Resource::new)
            .unwrap_or(resources::OUTLET);

        let alias = alias.unwrap_or_else(random_alias);

        // Check that there is no entry in the registry with the same alias
        if self.registry.outlets.contains_key(&alias).await {
            let message = format!("A TCP outlet with alias '{alias}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let check_credential = self.enable_credential_checks;
        let trust_context_id = if check_credential {
            Some(self.trust_context()?.id())
        } else {
            None
        };

        let access_control = self
            .access_control(&resource, &actions::HANDLE_MESSAGE, trust_context_id, None)
            .await?;

        let options = TcpOutletOptions::new().with_incoming_access_control(access_control);
        let options = if !check_credential {
            options.as_consumer(&self.api_transport_flow_control_id)
        } else {
            options
        };

        let options = if reachable_from_default_secure_channel {
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
}

impl NodeManagerWorker {
    pub(super) async fn get_inlets(&self, req: &RequestHeader) -> Response<InletList> {
        let registry = &self.node_manager.registry.inlets;
        Response::ok(req).body(InletList::new(
            registry
                .entries()
                .await
                .iter()
                .map(|(alias, info)| {
                    InletStatus::new(
                        &info.bind_addr,
                        info.worker_addr.to_string(),
                        alias,
                        None,
                        info.outlet_route.to_string(),
                    )
                })
                .collect(),
        ))
    }

    pub(super) async fn get_outlets(&self, req: &RequestHeader) -> Response<OutletList> {
        Response::ok(req).body(self.list_outlets().await)
    }

    pub(super) async fn create_inlet(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        let create_inlet_req: CreateInlet = dec.decode()?;
        self.create_inlet_impl(req, create_inlet_req, ctx).await
    }

    pub(super) async fn create_inlet_impl(
        &self,
        req: &RequestHeader,
        create_inlet_req: CreateInlet<'_>,
        ctx: &Context,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        info!("Handling request to create inlet portal");

        let listen_addr = create_inlet_req.listen_addr();
        let alias = create_inlet_req
            .alias()
            .map(|a| a.to_string())
            .unwrap_or_else(random_alias);
        debug! {
            prefix = %create_inlet_req.prefix_route(),
            suffix = %create_inlet_req.suffix_route(),
            listen_addr = %create_inlet_req.listen_addr(),
            outlet_addr = %create_inlet_req.outlet_addr(),
            %alias,
            "Creating inlet portal"
        }

        {
            let registry = &self.node_manager.registry.inlets;

            // Check that there is no entry in the registry with the same alias
            if registry.contains_key(&alias).await {
                return Err(Response::bad_request(
                    req,
                    &format!("A TCP inlet with alias '{alias}' already exists"),
                ));
            }

            // Check that there is no entry in the registry with the same TCP bind address
            if registry
                .values()
                .await
                .iter()
                .any(|inlet| inlet.bind_addr == listen_addr)
            {
                return Err(Response::bad_request(
                    req,
                    &format!("A TCP inlet with bind tcp address '{listen_addr}' already exists",),
                ));
            }
        }

        // The addressing scheme is very flexible. Typically the node connects to
        // the cloud via secure channel and the with another secure channel via
        // forwarder to the actual outlet on the target node. However it is also
        // possible that there is just a single secure channel used to go directly
        // to another node.

        let connection_instance = {
            let duration = create_inlet_req
                .wait_for_outlet_duration()
                .unwrap_or(Duration::from_secs(5));

            let connection = Connection::new(
                Arc::new(ctx.async_try_clone().await?),
                create_inlet_req.outlet_addr(),
            )
            .with_authorized_identity(create_inlet_req.authorized())
            .with_timeout(duration);

            NodeManager::connect(self.node_manager.clone(), connection).await?
        };

        let outlet_route = match local_multiaddr_to_route(&connection_instance.normalized_addr) {
            Some(route) => route,
            None => {
                return Err(Response::bad_request(req, "Invalid outlet route."));
            }
        };

        let outlet_route = route![
            create_inlet_req.prefix_route().clone(),
            outlet_route,
            create_inlet_req.suffix_route().clone()
        ];

        let resource = create_inlet_req
            .alias()
            .map(Resource::new)
            .unwrap_or(resources::INLET);

        let projects = self
            .node_manager
            .cli_state
            .projects
            .list()
            .map_err(|e| Response::bad_request(req, &e.to_string()))?;
        let projects = ProjectLookup::from_state(projects)
            .await
            .map_err(|e| Response::bad_request(req, &e.to_string()))?;
        let check_credential = self.node_manager.enable_credential_checks;
        let project_id = if check_credential {
            let pid = create_inlet_req
                .outlet_addr()
                .first()
                .and_then(|p| {
                    if let Some(p) = p.cast::<Project>() {
                        projects.get(&*p).map(|info| &*info.id)
                    } else {
                        None
                    }
                })
                .or_else(|| Some(self.node_manager.trust_context().ok()?.id()));
            if pid.is_none() {
                return Err(Response::bad_request(
                    req,
                    "Credential check requires a project or trust context",
                ));
            }
            pid
        } else {
            None
        };

        let access_control = self
            .node_manager
            .access_control(&resource, &actions::HANDLE_MESSAGE, project_id, None)
            .await?;

        let options = TcpInletOptions::new().with_incoming_access_control(access_control.clone());

        let res = self
            .node_manager
            .tcp_transport
            .create_inlet(listen_addr.clone(), outlet_route.clone(), options)
            .await;

        Ok(match res {
            Ok((socket_address, worker_addr)) => {
                //when using 0 port, the chosen port will be populated
                //in the returned socket address
                let listen_addr = socket_address.to_string();

                // TODO: Use better way to store inlets?
                self.node_manager
                    .registry
                    .inlets
                    .insert(
                        alias.clone(),
                        InletInfo::new(&listen_addr, Some(&worker_addr), &outlet_route),
                    )
                    .await;
                if !connection_instance.normalized_addr.is_empty() {
                    debug! {
                        %alias,
                        %listen_addr,
                        %worker_addr,
                        ping_addr = %connection_instance.transport_route,
                        "Creating session for TCP inlet"
                    };
                    let mut session = Session::new(connection_instance.transport_route.clone());

                    let ctx = Arc::new(ctx.async_try_clone().await?);
                    let repl = replacer(
                        self.node_manager.clone(),
                        connection_instance,
                        worker_addr.clone(),
                        listen_addr.clone(),
                        create_inlet_req.outlet_addr().clone(),
                        create_inlet_req.prefix_route().clone(),
                        create_inlet_req.suffix_route().clone(),
                        create_inlet_req.authorized(),
                        access_control.clone(),
                        ctx,
                    );
                    session.set_replacer(repl);
                    self.node_manager.add_session(session);
                }

                Response::ok(req).body(InletStatus::new(
                    listen_addr,
                    worker_addr.to_string(),
                    alias,
                    None,
                    outlet_route.to_string(),
                ))
            }
            Err(e) => {
                warn!(to = %create_inlet_req.outlet_addr(), err = %e, "Failed to create TCP inlet");
                return Err(Response::bad_request(
                    req,
                    &format!("Failed to create TCP inlet: {}", e),
                ));
            }
        })
    }

    pub(super) async fn delete_inlet<'a>(
        &self,
        req: &RequestHeader,
        alias: &'a str,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = self.node_manager.registry.inlets.remove(alias).await {
            debug!(%alias, "Successfully removed inlet from node registry");
            match self
                .node_manager
                .tcp_transport
                .stop_inlet(inlet_to_delete.worker_addr.clone())
                .await
            {
                Ok(_) => {
                    debug!(%alias, "Successfully stopped inlet");
                    Ok(Response::ok(req).body(InletStatus::new(
                        inlet_to_delete.bind_addr,
                        inlet_to_delete.worker_addr.to_string(),
                        alias,
                        None,
                        inlet_to_delete.outlet_route.to_string(),
                    )))
                }
                Err(e) => {
                    error!(%alias, "Failed to remove inlet from node registry");
                    Err(Response::internal_error(
                        req,
                        &format!("Failed to remove inlet with alias {alias}. {}", e),
                    ))
                }
            }
        } else {
            error!(%alias, "Inlet not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Inlet with alias {alias} not found"),
            ))
        }
    }

    pub(super) async fn show_inlet<'a>(
        &self,
        req: &RequestHeader,
        alias: &'a str,
    ) -> Result<Response<InletStatus>, Response<Error>> {
        info!(%alias, "Handling request to show inlet portal");
        if let Some(inlet_to_show) = self.node_manager.registry.inlets.get(alias).await {
            debug!(%alias, "Inlet not found in node registry");
            Ok(Response::ok(req).body(InletStatus::new(
                inlet_to_show.bind_addr.to_string(),
                inlet_to_show.worker_addr.to_string(),
                alias,
                None,
                inlet_to_show.outlet_route.to_string(),
            )))
        } else {
            error!(%alias, "Inlet not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Inlet with alias {alias} not found"),
            ))
        }
    }

    pub(super) async fn create_outlet(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        create_outlet: CreateOutlet,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        let CreateOutlet {
            socket_addr,
            worker_addr,
            alias,
            reachable_from_default_secure_channel,
            ..
        } = create_outlet;

        self.create_outlet_impl(
            ctx,
            req,
            socket_addr,
            worker_addr,
            alias,
            reachable_from_default_secure_channel,
        )
        .await
    }

    pub async fn create_outlet_impl(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        socket_addr: SocketAddr,
        worker_addr: Address,
        alias: Option<String>,
        reachable_from_default_secure_channel: bool,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self
            .node_manager
            .create_outlet(
                ctx,
                socket_addr,
                worker_addr,
                alias,
                reachable_from_default_secure_channel,
            )
            .await
        {
            Ok(outlet_status) => Ok(Response::ok(req).body(outlet_status)),
            Err(e) => Err(Response::bad_request(req, &format!("{e:?}"))),
        }
    }

    pub(super) async fn delete_outlet(
        &mut self,
        req: &RequestHeader,
        alias: &str,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.delete_outlet(alias).await {
            Ok(res) => match res {
                Some(outlet_info) => Ok(Response::ok(req).body(OutletStatus::new(
                    outlet_info.socket_addr,
                    outlet_info.worker_addr.clone(),
                    alias,
                    None,
                ))),
                None => Err(Response::bad_request(
                    req,
                    &format!("Outlet with alias {alias} not found"),
                )),
            },
            Err(e) => Err(Response::bad_request(req, &format!("{e:?}"))),
        }
    }

    pub(super) async fn show_outlet(
        &self,
        req: &RequestHeader,
        alias: &str,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        info!(%alias, "Handling request to show outlet portal");
        if let Some(outlet_to_show) = self.node_manager.registry.outlets.get(alias).await {
            debug!(%alias, "Outlet not found in node registry");
            Ok(Response::ok(req).body(OutletStatus::new(
                outlet_to_show.socket_addr,
                outlet_to_show.worker_addr.clone(),
                alias,
                None,
            )))
        } else {
            error!(%alias, "Outlet not found in the node registry");
            Err(Response::not_found(
                req,
                &format!("Outlet with alias {alias} not found"),
            ))
        }
    }

    pub async fn list_outlets(&self) -> OutletList {
        self.node_manager.list_outlets().await
    }
}

/// Create a session replacer.
///
/// This returns a function that accepts the previous ping address (e.g.
/// the secure channel worker address) and constructs the whole route
/// again.
#[allow(clippy::too_many_arguments)]
fn replacer(
    manager: Arc<NodeManager>,
    connection_instance: ConnectionInstance,
    inlet_address: Address,
    bind: String,
    addr: MultiAddr,
    prefix_route: Route,
    suffix_route: Route,
    auth: Option<Identifier>,
    access: Arc<dyn IncomingAccessControl>,
    ctx: Arc<Context>,
) -> Replacer {
    let connection_instance_arc = Arc::new(Mutex::new(connection_instance));
    let inlet_address_arc = Arc::new(Mutex::new(inlet_address));

    Box::new(move |previous_addr| {
        let addr = addr.clone();
        let auth = auth.clone();
        let bind = bind.clone();
        let node_manager = manager.clone();
        let access = access.clone();
        let ctx = ctx.clone();
        let connection_instance_arc = connection_instance_arc.clone();
        let inlet_address_arc = inlet_address_arc.clone();
        let inlet_address = inlet_address_arc.lock().unwrap().clone();
        let prefix_route = prefix_route.clone();
        let suffix_route = suffix_route.clone();
        let previous_connection_instance = connection_instance_arc.lock().unwrap().clone();

        Box::pin(async move {
            debug!(%previous_addr, %addr, "creating new tcp inlet");
            // The future that recreates the inlet:
            let f = async {
                //stop/delete previous secure channels
                for encryptor in &previous_connection_instance.secure_channel_encryptors {
                    let result = node_manager.delete_secure_channel(&ctx, encryptor).await;
                    if let Err(error) = result {
                        //we can't do much more
                        debug!("cannot delete secure channel `{encryptor}`: {error}");
                    }
                }
                if let Some(tcp_connection) = previous_connection_instance.tcp_connection.as_ref() {
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

                // Now a connection attempt is made:
                let connection = Connection::new(ctx.clone(), &addr)
                    .with_authorized_identity(auth)
                    .with_timeout(MAX_CONNECT_TIME);

                let new_connection_instance =
                    NodeManager::connect(node_manager.clone(), connection).await?;

                *connection_instance_arc.lock().unwrap() = new_connection_instance.clone();

                //we expect a fully normalized MultiAddr
                let normalized_route = route![
                    prefix_route,
                    local_multiaddr_to_route(&new_connection_instance.normalized_addr.clone())
                        .ok_or_else(|| ApiError::core("invalid normalized address"))?,
                    suffix_route
                ];

                let options = TcpInletOptions::new().with_incoming_access_control(access);

                // Finally attempt to create a new inlet using the new route:
                let new_inlet_address = node_manager
                    .tcp_transport
                    .create_inlet(bind, normalized_route, options)
                    .await?
                    .1;
                *inlet_address_arc.lock().unwrap() = new_inlet_address;

                Ok(new_connection_instance.transport_route.clone())
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
