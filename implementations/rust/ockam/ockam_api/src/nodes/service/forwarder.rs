use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use minicbor::Decoder;

use ockam::remote::RemoteForwarder;
use ockam::{Address, Result};
use ockam_core::api::{Error, Id, Request, Response, Status};
use ockam_core::AsyncTryClone;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Secure, Tcp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse, CredentialExchangeMode,
    DeleteSecureChannelRequest,
};
use crate::nodes::NodeManager;
use crate::session::Session;
use crate::{multiaddr_to_addr, multiaddr_to_route, try_address_to_multiaddr};

const MAX_RECOVERY_TIME: Duration = Duration::from_secs(10);
const MAX_CONNECT_TIME: Duration = Duration::from_secs(5);
const IDENTITY: &str = "authorized_identity";

impl NodeManager {
    pub(super) async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        rid: Id,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let req: CreateForwarder = dec.decode()?;

        debug!(addr = %req.address(), alias = ?req.alias(), "Handling CreateForwarder request");

        let addr = self.connect(&req).await?;
        let route = multiaddr_to_route(&addr)
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
            if f.is_ok() {
                let c = Arc::new(ctx.async_try_clone().await?);
                let mut s = Session::new(addr);
                if let Some(id) = req.authorized() {
                    // Save the authenticated identity so that we can use it if the
                    // secure channel needs to be recreated:
                    s.put(IDENTITY, id)
                }
                let this = ctx.address();
                enable_recovery(
                    &mut s,
                    this,
                    c,
                    req.address().clone(),
                    req.alias().map(|a| a.to_string()),
                    self.projects.clone(),
                );
                self.sessions.lock().unwrap().add(s);
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

    /// Resolve project ID (if any) and create secure channel if necessary.
    async fn connect(&mut self, req: &CreateForwarder<'_>) -> Result<MultiAddr> {
        if let Some(p) = req.address().first() {
            if p.code() == Project::CODE {
                let p = p
                    .cast::<Project>()
                    .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                let (mut a, i) = resolve_project(&self.projects, &p)?;
                a.try_extend(req.address().iter().skip(1))?;
                debug!(addr = %a, "creating secure channel");
                let r =
                    multiaddr_to_route(&a).ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                let i = Some(vec![i]);
                let m = CredentialExchangeMode::Oneway;
                let a = self.create_secure_channel_impl(r, i, m, None).await?;
                return try_address_to_multiaddr(&a);
            }
        }
        if req.address().matches(
            0,
            &[
                Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]),
                Tcp::CODE.into(),
                Secure::CODE.into(),
            ],
        ) {
            debug!(addr = %req.address(), "creating secure channel");
            let r = multiaddr_to_route(req.address())
                .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
            let i = req.authorized().map(|i| vec![i]);
            let m = CredentialExchangeMode::Oneway;
            let a = self.create_secure_channel_impl(r, i, m, None).await?;
            return try_address_to_multiaddr(&a);
        }
        Ok(req.address().clone())
    }
}

fn resolve_project(
    set: &BTreeMap<String, ProjectLookup>,
    name: &str,
) -> Result<(MultiAddr, IdentityIdentifier)> {
    if let Some(info) = set.get(name) {
        Ok((info.node_route.clone(), info.identity_id.clone()))
    } else {
        Err(ApiError::message(format!("project {name} not found")))
    }
}

/// Configure the session for automatic recovery.
fn enable_recovery(
    session: &mut Session,
    manager: Address,
    ctx: Arc<Context>,
    addr: MultiAddr,
    alias: Option<String>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
) {
    let auth = session.get::<IdentityIdentifier>(IDENTITY).cloned();
    session.set_replacement(move |prev| {
        let ctx = ctx.clone();
        let addr = addr.clone();
        let alias = alias.clone();
        let auth = auth.clone();
        let manager = manager.clone();
        let projects = projects.clone();
        Box::pin(async move {
            debug!(%prev, %addr, "creating new remote forwarder");
            let f = async {
                let a = if let Some(p) = addr.first() {
                    if p.code() == Project::CODE {
                        let p = p
                            .cast::<Project>()
                            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                        let (mut a, i) = resolve_project(&projects, &p)?;
                        a.try_extend(addr.iter().skip(1))?;
                        replace_sec_chan(&ctx, &manager, &prev, &a, Some(i)).await?
                    } else if addr.matches(
                        0,
                        &[
                            Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]),
                            Tcp::CODE.into(),
                            Secure::CODE.into(),
                        ],
                    ) {
                        replace_sec_chan(&ctx, &manager, &prev, &addr, auth).await?
                    } else {
                        addr.clone()
                    }
                } else {
                    addr.clone()
                };
                let r = multiaddr_to_route(&a)
                    .ok_or_else(|| ApiError::message(format!("invalid multiaddr: {a}")))?;
                if let Some(alias) = &alias {
                    RemoteForwarder::create_static(&ctx, r, alias).await?;
                } else {
                    RemoteForwarder::create(&ctx, r).await?;
                }
                Ok(a)
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

async fn replace_sec_chan(
    ctx: &Context,
    manager: &Address,
    prev: &MultiAddr,
    addr: &MultiAddr,
    auth: Option<IdentityIdentifier>,
) -> Result<MultiAddr> {
    debug!(%addr, %prev, "recreating secure channel");
    let req = {
        let a = multiaddr_to_addr(prev)
            .ok_or_else(|| ApiError::message(format!("could not map to address: {prev}")))?;
        DeleteSecureChannelRequest::new(&a)
    };
    let req = Request::delete("/node/secure_channel").body(req).to_vec()?;
    let vec: Vec<u8> = ctx.send_and_receive(manager.clone(), req).await?;
    let mut d = Decoder::new(&vec);
    let res: Response = d.decode()?;
    if res.status() != Some(Status::Ok) && res.has_body() {
        let e: Error = d.decode()?;
        debug!(%addr, %prev, err = ?e.message(), "failed to delete secure channel");
    }
    let auth = auth.map(|a| vec![a]);
    let mut req = CreateSecureChannelRequest::new(addr, auth, CredentialExchangeMode::Oneway);
    req.timeout = Some(MAX_CONNECT_TIME);
    let req = Request::post("/node/secure_channel").body(req).to_vec()?;
    let vec: Vec<u8> = ctx.send_and_receive(manager.clone(), req).await?;
    let mut d = Decoder::new(&vec);
    let res: Response = d.decode()?;
    if res.status() != Some(Status::Ok) {
        if res.has_body() {
            let e: Error = d.decode()?;
            warn!(%addr, %prev, err = ?e.message(), "failed to create secure channel");
        }
        return Err(ApiError::generic("error creating secure channel"));
    }
    let res: CreateSecureChannelResponse = d.decode()?;
    res.addr()
}
