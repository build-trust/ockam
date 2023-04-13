use std::collections::BTreeMap;
use std::sync::Arc;

use minicbor::Decoder;
use ockam::compat::asynchronous::RwLock;
use ockam::compat::sync::Mutex;
use ockam::identity::IdentityIdentifier;
use ockam::remote::{RemoteForwarder, RemoteForwarderInfo, RemoteForwarderOptions};
use ockam::Result;
use ockam_core::api::{Id, Request, Response, ResponseBuilder, Status};
use ockam_core::AsyncTryClone;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::connection::{Connection, ConnectionInstance};
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::session::sessions::{Replacer, Session};
use crate::session::sessions::{MAX_CONNECT_TIME, MAX_RECOVERY_TIME};

use super::{NodeManager, NodeManagerWorker};

impl NodeManagerWorker {
    pub(super) async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        rid: Id,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let manager = self.node_manager.clone();
        let req: CreateForwarder = dec.decode()?;

        debug!(addr = %req.address(), alias = ?req.alias(), "Handling CreateForwarder request");

        let connection = Connection::new(ctx, req.address())
            .with_authorized_identity(req.authorized())
            .add_default_consumers();

        let connection_instance =
            NodeManager::connect(self.node_manager.clone(), connection).await?;

        let options = RemoteForwarderOptions::new();

        let route = local_multiaddr_to_route(&connection_instance.normalized_addr)
            .ok_or_else(|| ApiError::message("invalid address: {addr}"))?;

        let forwarder = if req.at_rust_node() {
            if let Some(alias) = req.alias() {
                RemoteForwarder::create_static_without_heartbeats(ctx, route, alias, options).await
            } else {
                RemoteForwarder::create(ctx, route, options).await
            }
        } else {
            let result = if let Some(alias) = req.alias() {
                RemoteForwarder::create_static(ctx, route, alias, options).await
            } else {
                RemoteForwarder::create(ctx, route, options).await
            };
            if result.is_ok() && !connection_instance.transport_route.is_empty() {
                let ctx = Arc::new(ctx.async_try_clone().await?);
                let repl = replacer(
                    manager,
                    ctx,
                    connection_instance.clone(),
                    req.address().clone(),
                    req.alias().map(|a| a.to_string()),
                    req.authorized(),
                );
                let node_manager = self.node_manager.write().await;
                let mut session = Session::new(connection_instance.transport_route);
                session.set_replacer(repl);
                node_manager.sessions.lock().unwrap().add(session);
            }
            result
        };

        match forwarder {
            Ok(info) => {
                let registry_info = info.clone();
                let registry_remote_address = registry_info.remote_address().to_string();
                let res_body = ForwarderInfo::from(info);
                let mut node_manager = self.node_manager.write().await;
                node_manager
                    .registry
                    .forwarders
                    .insert(registry_remote_address, registry_info);

                debug!(
                    forwarding_route = %res_body.forwarding_route(),
                    remote_address = %res_body.remote_address_ma()?,
                    "CreateForwarder request processed, sending back response"
                );
                Ok(Response::ok(rid).body(res_body).to_vec()?)
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Ok(Response::builder(rid, Status::InternalServerError)
                    .body(err.to_string())
                    .to_vec()?)
            }
        }
    }

    pub(super) async fn delete_forwarder<'a>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        remote_address: &'a str,
    ) -> Result<ResponseBuilder<Option<ForwarderInfo<'a>>>> {
        let mut node_manager = self.node_manager.write().await;

        debug!(%remote_address , "Handling DeleteForwarder request");

        if let Some(forwarder_to_delete) = node_manager.registry.forwarders.remove(remote_address) {
            debug!(%remote_address, "Successfully removed forwarder from node registry");

            let was_stopped = ctx
                .stop_worker(forwarder_to_delete.worker_address().clone())
                .await
                .is_ok();

            if was_stopped {
                debug!(%remote_address, "Successfully stopped forwarder");
                Ok(Response::ok(req.id())
                    .body(Some(ForwarderInfo::from(forwarder_to_delete.to_owned()))))
            } else {
                error!(%remote_address, "Failed to delete forwarder from node registry");
                Ok(Response::internal_error(req.id())
                    .body(Some(ForwarderInfo::from(forwarder_to_delete.to_owned()))))
            }
        } else {
            error!(%remote_address, "Forwarder not found in the node registry");
            Ok(Response::not_found(req.id()).body(None))
        }
    }

    pub(super) async fn show_forwarder<'a>(
        &mut self,
        req: &Request<'_>,
        remote_address: &'a str,
    ) -> Result<ResponseBuilder<Option<ForwarderInfo<'a>>>> {
        debug!("Handling ShowForwarder request");
        let node_manager = self.node_manager.read().await;
        if let Some(forwarder_to_show) = node_manager.registry.forwarders.get(remote_address) {
            debug!(%remote_address, "Forwarder not found in node registry");
            Ok(
                Response::ok(req.id())
                    .body(Some(ForwarderInfo::from(forwarder_to_show.to_owned()))),
            )
        } else {
            error!(%remote_address, "Forwarder not found in the node registry");

            Ok(Response::not_found(req.id()).body(None))
        }
    }

    pub(super) async fn get_forwarders<'a>(
        &mut self,
        req: &Request<'a>,
        registry: &'a BTreeMap<String, RemoteForwarderInfo>,
    ) -> ResponseBuilder<Vec<ForwarderInfo<'a>>> {
        debug!("Handling ListForwarders request");
        Response::ok(req.id()).body(
            registry
                .iter()
                .map(|(_, registry_info)| ForwarderInfo::from(registry_info.to_owned()))
                .collect(),
        )
    }
}

/// Create a session replacer.
///
/// This returns a function that accepts the previous ping address (e.g.
/// the secure channel worker address) and constructs the whole route
/// again.
fn replacer(
    manager: Arc<RwLock<NodeManager>>,
    ctx: Arc<Context>,
    connection_instance: ConnectionInstance,
    addr: MultiAddr,
    alias: Option<String>,
    auth: Option<IdentityIdentifier>,
) -> Replacer {
    let connection_instance_arc = Arc::new(Mutex::new(connection_instance));
    Box::new(move |prev_route| {
        let ctx = ctx.clone();
        let addr = addr.clone();
        let alias = alias.clone();
        let auth = auth.clone();
        let node_manager_arc = manager.clone();
        let connection_instance_arc = connection_instance_arc.clone();
        let previous_connection_instance = connection_instance_arc.lock().unwrap().clone();

        Box::pin(async move {
            debug!(%prev_route, %addr, "creating new remote forwarder");

            let f = async {
                let mut node_manager = node_manager_arc.write().await;
                for encryptor in &previous_connection_instance.secure_channel_encryptors {
                    if let Err(error) = node_manager.delete_secure_channel(&ctx, encryptor).await {
                        //not much we can do about it
                        debug!("cannot delete secure channel `{encryptor}`: {error}");
                    }
                }
                if let Some(tcp_worker) = previous_connection_instance.tcp_worker.as_ref() {
                    if let Err(error) = node_manager.tcp_transport.disconnect(tcp_worker).await {
                        debug!("cannot stop tcp worker `{tcp_worker}`: {error}");
                    }
                }
                drop(node_manager);

                let connection = Connection::new(ctx.as_ref(), &addr)
                    .with_authorized_identity(auth)
                    .with_timeout(MAX_CONNECT_TIME)
                    .add_default_consumers();

                let new_connection_instance =
                    NodeManager::connect(node_manager_arc, connection).await?;

                *connection_instance_arc.lock().unwrap() = new_connection_instance.clone();

                let route = local_multiaddr_to_route(&new_connection_instance.normalized_addr)
                    .ok_or_else(|| {
                        ApiError::message(format!(
                            "invalid multiaddr: {}",
                            &new_connection_instance.normalized_addr
                        ))
                    })?;

                let options = RemoteForwarderOptions::new();
                if let Some(alias) = &alias {
                    RemoteForwarder::create_static(&ctx, route, alias, options).await?;
                } else {
                    RemoteForwarder::create(&ctx, route, options).await?;
                }

                Ok(new_connection_instance.transport_route)
            };
            match timeout(MAX_RECOVERY_TIME, f).await {
                Err(_) => {
                    warn!(%addr, "timeout creating new remote forwarder");
                    Err(ApiError::generic("timeout"))
                }
                Ok(Err(e)) => {
                    warn!(%addr, err = %e, "error creating new remote forwarder");
                    Err(e)
                }
                Ok(Ok(a)) => Ok(a),
            }
        })
    })
}
