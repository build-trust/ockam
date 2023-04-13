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
use minicbor::Decoder;
use ockam::compat::tokio::time::timeout;
use ockam::identity::IdentityIdentifier;
use ockam::{Address, AsyncTryClone, Result};

use ockam_abac::Resource;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{route, IncomingAccessControl, Route};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;
use ockam_transport_tcp::{TcpInletOptions, TcpOutletOptions};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use super::{NodeManager, NodeManagerWorker};

impl NodeManagerWorker {
    pub(super) fn get_inlets<'a>(
        &self,
        req: &Request<'a>,
        inlet_registry: &'a BTreeMap<String, InletInfo>,
    ) -> ResponseBuilder<InletList<'a>> {
        Response::ok(req.id()).body(InletList::new(
            inlet_registry
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

    pub(super) fn get_outlets<'a>(
        &self,
        req: &Request<'_>,
        outlet_registry: &'a BTreeMap<String, OutletInfo>,
    ) -> ResponseBuilder<OutletList<'a>> {
        Response::ok(req.id()).body(OutletList::new(
            outlet_registry
                .iter()
                .map(|(alias, info)| {
                    OutletStatus::new(&info.tcp_addr, info.worker_addr.to_string(), alias, None)
                })
                .collect(),
        ))
    }

    pub(super) async fn create_inlet<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<InletStatus<'a>>> {
        let rid = req.id();
        let req: CreateInlet = dec.decode()?;
        self.create_inlet_impl(rid, req, ctx).await
    }

    pub(super) async fn create_inlet_impl<'a>(
        &mut self,
        rid: ockam_core::api::Id,
        req: CreateInlet<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<InletStatus<'a>>> {
        let manager = self.node_manager.clone();

        let listen_addr = req.listen_addr().to_string();
        let alias = req
            .alias()
            .map(|a| a.to_string())
            .unwrap_or_else(random_alias);

        info!("Handling request to create inlet portal");

        debug! {
            listen_addr = %req.listen_addr(),
            outlet_addr = %req.outlet_addr(),
            %alias,
            "Creating inlet portal"
        }

        // The addressing scheme is very flexible. Typically the node connects to
        // the cloud via secure channel and the with another secure channel via
        // forwarder to the actual outlet on the target node. However it is also
        // possible that there is just a single secure channel used to go directly
        // to another node.

        let connection_instance = {
            let duration = req
                .wait_for_outlet_duration()
                .unwrap_or(Duration::from_secs(5));

            let connection = Connection::new(ctx, req.outlet_addr())
                .with_authorized_identity(req.authorized())
                .with_timeout(duration);

            NodeManager::connect(manager.clone(), connection).await?
        };

        let outlet_route = match local_multiaddr_to_route(&connection_instance.normalized_addr) {
            Some(route) => route,
            None => {
                return Ok(Response::bad_request(rid)
                    .body(InletStatus::bad_request("invalid outlet route")))
            }
        };

        // prefix services needs to be part of the session
        // suffix services are remote so we can safely ignore them
        for address in req.prefix_route().iter() {
            connection_instance.add_consumer(ctx, address);
        }

        let outlet_route = route![
            req.prefix_route().clone(),
            outlet_route,
            req.suffix_route().clone()
        ];

        let resource = req.alias().map(Resource::new).unwrap_or(resources::INLET);

        let mut node_manager = self.node_manager.write().await;
        let check_credential = node_manager.enable_credential_checks;
        let project_id = if check_credential {
            let pid = req
                .outlet_addr()
                .first()
                .and_then(|p| {
                    if let Some(p) = p.cast::<Project>() {
                        node_manager.projects.get(&*p).map(|info| &*info.id)
                    } else {
                        None
                    }
                })
                .or_else(|| Some(node_manager.trust_context().ok()?.id()));
            if pid.is_none() {
                return Err(ApiError::generic("credential check requires project"));
            }
            pid
        } else {
            None
        };

        let access_control = node_manager
            .access_control(&resource, &actions::HANDLE_MESSAGE, project_id, None)
            .await?;

        let options = TcpInletOptions::new().with_incoming_access_control(access_control.clone());

        let res = node_manager
            .tcp_transport
            .create_inlet(listen_addr.clone(), outlet_route.clone(), options)
            .await;

        Ok(match res {
            Ok((socket_address, worker_addr)) => {
                //when using 0 port, the chosen port will be populated
                //in the returned socket address
                let listen_addr = socket_address.to_string();

                // TODO: Use better way to store inlets?
                node_manager.registry.inlets.insert(
                    alias.clone(),
                    InletInfo::new(&listen_addr, Some(&worker_addr), &outlet_route),
                );
                if !connection_instance.normalized_addr.is_empty() {
                    let mut session = Session::new(connection_instance.transport_route.clone());

                    let ctx = Arc::new(ctx.async_try_clone().await?);
                    let repl = replacer(
                        manager,
                        connection_instance,
                        worker_addr.clone(),
                        listen_addr.clone(),
                        req.outlet_addr().clone(),
                        req.prefix_route().clone(),
                        req.suffix_route().clone(),
                        req.authorized(),
                        access_control.clone(),
                        ctx,
                    );
                    session.set_replacer(repl);
                    node_manager.sessions.lock().unwrap().add(session);
                }

                Response::ok(rid).body(InletStatus::new(
                    listen_addr,
                    worker_addr.to_string(),
                    alias,
                    None,
                    outlet_route.to_string(),
                ))
            }
            Err(e) => {
                warn!(to = %req.outlet_addr(), err = %e, "failed to create tcp inlet");
                // TODO: Use better way to store inlets?
                node_manager.registry.inlets.insert(
                    alias.clone(),
                    InletInfo::new(&listen_addr, None, &outlet_route),
                );

                Response::bad_request(rid).body(InletStatus::new(
                    listen_addr,
                    "",
                    alias,
                    Some(e.to_string().into()),
                    outlet_route.to_string(),
                ))
            }
        })
    }

    pub(super) async fn delete_inlet<'a>(
        &mut self,
        req: &Request<'_>,
        alias: &'a str,
    ) -> Result<ResponseBuilder<InletStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;

        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = node_manager.registry.inlets.remove(alias) {
            debug!(%alias, "Sucessfully removed inlet from node registry");
            let was_stopped = node_manager
                .tcp_transport
                .stop_inlet(inlet_to_delete.worker_addr.clone())
                .await
                .is_ok();
            if was_stopped {
                debug!(%alias, "Successfully stopped inlet");
                Ok(Response::ok(req.id()).body(InletStatus::new(
                    inlet_to_delete.bind_addr,
                    inlet_to_delete.worker_addr.to_string(),
                    alias,
                    None,
                    inlet_to_delete.outlet_route.to_string(),
                )))
            } else {
                error!(%alias, "Failed to remove inlet from node registry");
                Ok(Response::internal_error(req.id()).body(InletStatus::new(
                    inlet_to_delete.bind_addr,
                    inlet_to_delete.worker_addr.to_string(),
                    alias,
                    Some(format!("Failed to remove inlet with alias {alias}").into()),
                    inlet_to_delete.outlet_route.to_string(),
                )))
            }
        } else {
            error!(%alias, "Inlet not found in the node registry");
            Ok(Response::not_found(req.id()).body(InletStatus::new(
                "".to_string(),
                "".to_string(),
                alias,
                Some(format!("Inlet with alias {alias} not found").into()),
                "".to_string(),
            )))
        }
    }

    pub(super) async fn show_inlet<'a>(
        &mut self,
        req: &Request<'_>,
        alias: &'a str,
    ) -> Result<ResponseBuilder<InletStatus<'a>>> {
        let node_manager = self.node_manager.read().await;

        info!(%alias, "Handling request to show inlet portal");
        if let Some(inlet_to_show) = node_manager.registry.inlets.get(alias) {
            debug!(%alias, "Inlet not found in node registry");
            Ok(Response::ok(req.id()).body(InletStatus::new(
                inlet_to_show.bind_addr.to_string(),
                inlet_to_show.worker_addr.to_string(),
                alias,
                None,
                inlet_to_show.outlet_route.to_string(),
            )))
        } else {
            error!(%alias, "Inlet not found in the node registry");
            Ok(Response::not_found(req.id()).body(InletStatus::new(
                "".to_string(),
                "".to_string(),
                alias,
                Some(format!("Inlet with alias {alias} not found").into()),
                "".to_string(),
            )))
        }
    }

    pub(super) async fn create_outlet<'a>(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<OutletStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateOutlet {
            tcp_addr,
            worker_addr,
            alias,
            ..
        } = dec.decode()?;
        let tcp_addr = tcp_addr.to_string();
        let resource = alias
            .as_deref()
            .map(Resource::new)
            .unwrap_or(resources::OUTLET);
        let alias = alias.map(|a| a.0.into()).unwrap_or_else(random_alias);

        info!("Handling request to create outlet portal");
        let worker_addr = Address::from(worker_addr.as_ref());

        let check_credential = node_manager.enable_credential_checks;
        let trust_context_id = if check_credential {
            Some(node_manager.trust_context()?.id())
        } else {
            None
        };

        let access_control = node_manager
            .access_control(&resource, &actions::HANDLE_MESSAGE, trust_context_id, None)
            .await?;
        let options = TcpOutletOptions::new().with_incoming_access_control(access_control);

        // Accept messages from the default secure channel listener
        let options = if let Some(flow_control_id) = ctx
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
        {
            options.as_consumer(
                &flow_control_id,
                FlowControlPolicy::SpawnerAllowMultipleMessages,
            )
        } else {
            options
        };

        let res = node_manager
            .tcp_transport
            .create_outlet(worker_addr.clone(), tcp_addr.clone(), options)
            .await;

        Ok(match res {
            Ok(_) => {
                // TODO: Use better way to store outlets?
                node_manager.registry.outlets.insert(
                    alias.clone(),
                    OutletInfo::new(&tcp_addr, Some(&worker_addr)),
                );

                Response::ok(req.id()).body(OutletStatus::new(
                    tcp_addr,
                    worker_addr.to_string(),
                    alias,
                    None,
                ))
            }
            Err(e) => {
                // TODO: Use better way to store outlets?
                node_manager
                    .registry
                    .outlets
                    .insert(alias.clone(), OutletInfo::new(&tcp_addr, None));

                Response::bad_request(req.id()).body(OutletStatus::new(
                    tcp_addr,
                    worker_addr.to_string(),
                    alias,
                    Some(e.to_string().into()),
                ))
            }
        })
    }

    pub(super) async fn delete_outlet<'a>(
        &mut self,
        req: &Request<'_>,
        alias: &'a str,
    ) -> Result<ResponseBuilder<OutletStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;

        info!(%alias, "Handling request to delete outlet portal");
        if let Some(outlet_to_delete) = node_manager.registry.outlets.remove(alias) {
            debug!(%alias, "Successfully removed outlet from node registry");
            let was_stopped = node_manager
                .tcp_transport
                .stop_outlet(outlet_to_delete.worker_addr.clone())
                .await
                .is_ok();
            if was_stopped {
                debug!(%alias, "Successfully stopped outlet");
                Ok(Response::ok(req.id()).body(OutletStatus::new(
                    outlet_to_delete.tcp_addr,
                    outlet_to_delete.worker_addr.to_string(),
                    alias,
                    None,
                )))
            } else {
                error!(%alias, "Failed to remove outlet from node registry");
                Ok(Response::internal_error(req.id()).body(OutletStatus::new(
                    outlet_to_delete.tcp_addr,
                    outlet_to_delete.worker_addr.to_string(),
                    alias,
                    Some(format!("Failed to remove outlet with alias {alias}").into()),
                )))
            }
        } else {
            error!(%alias, "Outlet not found in the node registry");
            Ok(Response::not_found(req.id()).body(OutletStatus::new(
                "".to_string(),
                "".to_string(),
                alias,
                Some(format!("Outlet with alias {alias} not found").into()),
            )))
        }
    }

    pub(super) async fn show_outlet<'a>(
        &mut self,
        req: &Request<'_>,
        alias: &'a str,
    ) -> Result<ResponseBuilder<OutletStatus<'a>>> {
        let node_manager = self.node_manager.read().await;

        info!(%alias, "Handling request to show outlet portal");
        if let Some(outlet_to_show) = node_manager.registry.outlets.get(alias) {
            debug!(%alias, "Outlet not found in node registry");
            Ok(Response::ok(req.id()).body(OutletStatus::new(
                outlet_to_show.tcp_addr.to_string(),
                outlet_to_show.worker_addr.to_string(),
                alias,
                None,
            )))
        } else {
            error!(%alias, "Outlet not found in the node registry");
            Ok(Response::not_found(req.id()).body(OutletStatus::new(
                "".to_string(),
                "".to_string(),
                alias,
                Some(format!("Outlet with alias {alias} not found").into()),
            )))
        }
    }
}

/// Create a session replacer.
///
/// This returns a function that accepts the previous ping address (e.g.
/// the secure channel worker address) and constructs the whole route
/// again.
#[allow(clippy::too_many_arguments)]
fn replacer(
    manager: Arc<RwLock<NodeManager>>,
    connection_instance: ConnectionInstance,
    inlet_address: Address,
    bind: String,
    addr: MultiAddr,
    prefix_route: Route,
    suffix_route: Route,
    auth: Option<IdentityIdentifier>,
    access: Arc<dyn IncomingAccessControl>,
    ctx: Arc<Context>,
) -> Replacer {
    let connection_instance_arc = Arc::new(Mutex::new(connection_instance));
    let inlet_address_arc = Arc::new(Mutex::new(inlet_address));

    Box::new(move |previous_addr| {
        let addr = addr.clone();
        let auth = auth.clone();
        let bind = bind.clone();
        let node_manager_arc = manager.clone();
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
                let mut node_manager = node_manager_arc.write().await;
                //stop/delete previous secure channels
                for encryptor in &previous_connection_instance.secure_channel_encryptors {
                    let result = node_manager.delete_secure_channel(&ctx, encryptor).await;
                    if let Err(error) = result {
                        //we can't do much more
                        debug!("cannot delete secure channel `{encryptor}`: {error}");
                    }
                }
                if let Some(tcp_worker) = previous_connection_instance.tcp_worker.as_ref() {
                    if let Err(error) = node_manager.tcp_transport.disconnect(tcp_worker).await {
                        debug!("cannot stop tcp worker `{tcp_worker}`: {error}");
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
                drop(node_manager);

                // Now a connection attempt is made:
                let connection = Connection::new(ctx.as_ref(), &addr)
                    .with_authorized_identity(auth)
                    .with_timeout(MAX_CONNECT_TIME);

                let new_connection_instance =
                    NodeManager::connect(node_manager_arc.clone(), connection).await?;

                *connection_instance_arc.lock().unwrap() = new_connection_instance.clone();

                for address in prefix_route.iter() {
                    new_connection_instance.add_consumer(&ctx, address);
                }

                //we expect a fully normalized MultiAddr
                let normalized_route = route![
                    prefix_route,
                    local_multiaddr_to_route(&new_connection_instance.normalized_addr.clone())
                        .ok_or_else(|| ApiError::generic("invalid normalized address"))?,
                    suffix_route
                ];

                let node_manager = node_manager_arc.write().await;

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
                    Err(ApiError::generic("timeout"))
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
