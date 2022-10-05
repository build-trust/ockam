use std::sync::Arc;

use minicbor::Decoder;

use ockam::compat::asynchronous::RwLock;
use ockam::remote::RemoteForwarder;
use ockam::Result;
use ockam_core::api::{Id, Response, Status};
use ockam_core::AsyncTryClone;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::session::util;
use crate::session::{Replacer, Session};
use crate::{multiaddr_to_route, try_multiaddr_to_addr};

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

        let (sec_chan, suffix) = node_manager
            .connect(req.address(), req.authorized(), None)
            .await?;

        let full = sec_chan.clone().try_with(&suffix)?;
        let route = multiaddr_to_route(&full)
            .ok_or_else(|| ApiError::message("invalid address: {addr}"))?;

        let forwarder = if req.at_rust_node() {
            if let Some(alias) = req.alias() {
                RemoteForwarder::create_static_without_heartbeats(ctx, route, alias).await
            } else {
                RemoteForwarder::create(ctx, route).await
            }
        } else {
            let f = if let Some(alias) = req.alias() {
                RemoteForwarder::create_static(ctx, route, alias).await
            } else {
                RemoteForwarder::create(ctx, route).await
            };
            if f.is_ok() && !sec_chan.is_empty() {
                let ctx = Arc::new(ctx.async_try_clone().await?);
                let repl = replacer(
                    manager,
                    ctx,
                    req.address().clone(),
                    req.alias().map(|a| a.to_string()),
                    req.authorized(),
                );
                let mut s = Session::new(sec_chan);
                s.set_replacer(repl);
                node_manager.sessions.lock().unwrap().add(s);
            }
            f
        };

        match forwarder {
            Ok(info) => {
                let b = ForwarderInfo::from(info);
                debug!(
                    forwarding_route = %b.forwarding_route(),
                    remote_address = %b.remote_address(),
                    "CreateForwarder request processed, sending back response"
                );
                Ok(Response::ok(rid).body(b).to_vec()?)
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Ok(Response::builder(rid, Status::InternalServerError)
                    .body(err.to_string())
                    .to_vec()?)
            }
        }
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
                let timeout = Some(util::MAX_CONNECT_TIME);
                let (sec, rest) = this.connect(&addr, auth, timeout).await?;
                let a = sec.clone().try_with(&rest)?;
                let r = multiaddr_to_route(&a)
                    .ok_or_else(|| ApiError::message(format!("invalid multiaddr: {a}")))?;
                if let Some(alias) = &alias {
                    RemoteForwarder::create_static(&ctx, r, alias).await?;
                } else {
                    RemoteForwarder::create(&ctx, r).await?;
                }
                Ok(sec)
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
