use std::collections::BTreeMap;
use std::sync::Arc;

use minicbor::Decoder;

use ockam::remote::RemoteForwarder;
use ockam::{Address, Result};
use ockam_core::api::{Id, Response, Status};
use ockam_core::AsyncTryClone;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::secure_channel::CredentialExchangeMode;
use crate::session::util;
use crate::session::{Replacer, Session};

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        rid: Id,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let mut node_manager = self.node_manager.write().await;
        let req: CreateForwarder = dec.decode()?;

        debug!(addr = %req.address(), alias = ?req.alias(), "Handling CreateForwarder request");

        let (sec_chan, suffix) = node_manager
            .connect(
                req.address(),
                CredentialExchangeMode::Oneway,
                req.authorized(),
            )
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
                let this = ctx.address();
                let ctx = Arc::new(ctx.async_try_clone().await?);
                let repl = replacer(
                    this,
                    ctx,
                    req.address().clone(),
                    req.alias().map(|a| a.to_string()),
                    node_manager.projects.clone(),
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

/// Configure the session for automatic recovery.
fn replacer(
    manager: Address,
    ctx: Arc<Context>,
    addr: MultiAddr,
    alias: Option<String>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
    auth: Option<IdentityIdentifier>,
) -> Replacer {
    Box::new(move |prev| {
        let ctx = ctx.clone();
        let addr = addr.clone();
        let alias = alias.clone();
        let auth = auth.clone();
        let manager = manager.clone();
        let projects = projects.clone();
        Box::pin(async move {
            debug!(%prev, %addr, "creating new remote forwarder");
            let f = async {
                let (w, a) = if let Some(p) = addr.first() {
                    if p.code() == Project::CODE {
                        let p = p
                            .cast::<Project>()
                            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                        let (a, i) = util::resolve_project(&projects, &p)?;
                        util::delete_sec_chan(&ctx, &manager, &prev).await?;
                        let m = CredentialExchangeMode::Oneway;
                        let w = util::create_sec_chan(&ctx, &manager, &a, Some(i), m).await?;
                        (w, MultiAddr::default().try_with(addr.iter().skip(1))?)
                    } else if let Some(pos) = util::starts_with_host_tcp_secure(&addr) {
                        let (a, b) = addr.split(pos);
                        util::delete_sec_chan(&ctx, &manager, &prev).await?;
                        let m = CredentialExchangeMode::Oneway;
                        let w = util::create_sec_chan(&ctx, &manager, &a, auth, m).await?;
                        (w, b)
                    } else {
                        (MultiAddr::default(), addr.clone())
                    }
                } else {
                    (MultiAddr::default(), addr.clone())
                };
                let x = w.clone().try_with(&a)?;
                let r = multiaddr_to_route(&x)
                    .ok_or_else(|| ApiError::message(format!("invalid multiaddr: {a}")))?;
                if let Some(alias) = &alias {
                    RemoteForwarder::create_static(&ctx, r, alias).await?;
                } else {
                    RemoteForwarder::create(&ctx, r).await?;
                }
                Ok(w)
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
