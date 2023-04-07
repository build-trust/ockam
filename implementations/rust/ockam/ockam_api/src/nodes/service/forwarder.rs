use std::collections::BTreeMap;
use std::sync::Arc;

use minicbor::Decoder;

use ockam::compat::asynchronous::RwLock;
use ockam::remote::{RemoteForwarder, RemoteForwarderInfo, RemoteForwarderTrustOptions};
use ockam::Result;
use ockam_core::api::{Id, Request, Response, ResponseBuilder, Status};
use ockam_core::AsyncTryClone;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::session::util;
use crate::session::{Replacer, Session};
use crate::{local_multiaddr_to_route, try_multiaddr_to_addr};

use super::{NodeManager, NodeManagerWorker};

impl NodeManagerWorker {
    pub(super) async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        rid: Id,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let manager = self.node_manager.clone();
        let mut node_manager = self.node_manager.write().await;
        let req: CreateForwarder = dec.decode()?;

        debug!(addr = %req.address(), alias = ?req.alias(), "Handling CreateForwarder request");

        let connection = Connection::new(ctx, req.address())
            .with_authorized_identity(req.authorized())
            .add_default_consumers();

        let connection = node_manager.connect(connection).await?;

        let trust_options = RemoteForwarderTrustOptions::as_consumer_and_producer(
            &node_manager.message_flow_sessions,
        );

        let full = connection
            .secure_channel
            .clone()
            .try_with(&connection.suffix)?;
        let route = local_multiaddr_to_route(&full)
            .ok_or_else(|| ApiError::message("invalid address: {addr}"))?;

        let forwarder = if req.at_rust_node() {
            if let Some(alias) = req.alias() {
                RemoteForwarder::create_static_without_heartbeats(ctx, route, alias, trust_options)
                    .await
            } else {
                RemoteForwarder::create(ctx, route, trust_options).await
            }
        } else {
            let f = if let Some(alias) = req.alias() {
                RemoteForwarder::create_static(ctx, route, alias, trust_options).await
            } else {
                RemoteForwarder::create(ctx, route, trust_options).await
            };
            if f.is_ok() && !connection.secure_channel.is_empty() {
                let ctx = Arc::new(ctx.async_try_clone().await?);
                let repl = replacer(
                    manager,
                    ctx,
                    req.address().clone(),
                    req.alias().map(|a| a.to_string()),
                    req.authorized(),
                );
                let mut s = Session::new(connection.secure_channel);
                s.set_replacer(repl);
                node_manager.sessions.lock().unwrap().add(s);
            }
            f
        };

        match forwarder {
            Ok(info) => {
                let registry_info = info.clone();
                let registry_remote_address = registry_info.remote_address().to_string();
                let res_body = ForwarderInfo::from(info);
                node_manager
                    .registry
                    .forwarders
                    .insert(registry_remote_address, registry_info);

                debug!(
                    forwarding_route = %res_body.forwarding_route(),
                    remote_address = %res_body.remote_address(),
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
    addr: MultiAddr,
    alias: Option<String>,
    auth: Option<IdentityIdentifier>,
) -> Replacer {
    Box::new(move |prev| {
        let ctx = ctx.clone();
        let addr = addr.clone();
        let alias = alias.clone();
        let auth = auth.clone();
        let manager = manager.clone();
        Box::pin(async move {
            debug!(%prev, %addr, "creating new remote forwarder");
            let f = async {
                let prev = try_multiaddr_to_addr(&prev)?;
                let mut this = manager.write().await;
                let _ = this.delete_secure_channel(&prev).await;
                let connection = Connection::new(ctx.as_ref(), &addr)
                    .with_authorized_identity(auth)
                    .with_timeout(util::MAX_CONNECT_TIME)
                    .add_default_consumers();
                let connection = this.connect(connection).await?;
                let a = connection
                    .secure_channel
                    .clone()
                    .try_with(&connection.suffix)?;
                let r = local_multiaddr_to_route(&a)
                    .ok_or_else(|| ApiError::message(format!("invalid multiaddr: {a}")))?;

                let trust_options = RemoteForwarderTrustOptions::as_consumer_and_producer(
                    &this.message_flow_sessions,
                );

                if let Some(alias) = &alias {
                    RemoteForwarder::create_static(&ctx, r, alias, trust_options).await?;
                } else {
                    RemoteForwarder::create(&ctx, r, trust_options).await?;
                }

                Ok(connection.secure_channel)
            };
            match timeout(util::MAX_RECOVERY_TIME, f).await {
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
