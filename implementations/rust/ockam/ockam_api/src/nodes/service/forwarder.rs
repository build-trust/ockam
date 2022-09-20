use std::collections::HashMap;
use std::convert::identity;
use std::sync::Arc;
use std::time::Duration;

use either::Either;
use minicbor::Decoder;

use ockam::remote::RemoteForwarder;
use ockam::{Address, Result, Route};
use ockam_core::api::{Error, Id, Request, Response, Status};
use ockam_core::AsyncTryClone;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Service, Tcp};
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::tokio::time::timeout;
use ockam_node::Context;

use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse, CredentialExchangeMode,
    DeleteSecureChannelRequest,
};
use crate::nodes::NodeManager;
use crate::session::Session;

const MAX_RECOVERY_TIME: Duration = Duration::from_secs(10);
const MAX_CONNECT_TIME: Duration = Duration::from_secs(5);

impl NodeManager {
    pub(super) async fn create_forwarder(
        &mut self,
        ctx: &mut Context,
        rid: Id,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let req: CreateForwarder = dec.decode()?;

        debug!(addr = %req.address, alias = ?req.alias, "Handling CreateForwarder request");

        let forwarder = match req.alias {
            Some(alias) => {
                let auth = Arc::new(req.identities);
                let addr = connect(self, &req.address, &auth, req.mode).await?;
                let alias = alias.to_string();
                let fwdr = if req.at_rust_node {
                    let route = addr.either(Route::from, identity);
                    RemoteForwarder::create_static_without_heartbeats(ctx, route, alias).await
                } else {
                    // If connect returned an address it is a secure channel to an api node and
                    // automatic recovery of a remote forwarder is enabled.
                    match addr {
                        Either::Left(a) => {
                            let r = Route::from(a.clone());
                            let f = RemoteForwarder::create_static(ctx, r, alias.clone()).await;
                            if f.is_ok() {
                                let c = Arc::new(ctx.async_try_clone().await?);
                                let mut s = Session::new(a);
                                let this = self.address.clone();
                                enable_recovery(
                                    &mut s,
                                    this,
                                    c,
                                    req.address,
                                    alias.to_string(),
                                    auth,
                                    req.mode,
                                );
                                self.sessions.lock().unwrap().add(s);
                            }
                            f
                        }
                        Either::Right(r) => {
                            RemoteForwarder::create_static(ctx, r, alias.clone()).await
                        }
                    }
                };
                fwdr
            }
            None => {
                let r = multiaddr_to_route(&req.address)
                    .ok_or_else(|| ApiError::generic("invalid multiaddress"))?;
                RemoteForwarder::create(ctx, r).await
            }
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

/// Get address prefix corresponding to the API host (if any).
#[rustfmt::skip]
fn api_host(input: &MultiAddr) -> Option<MultiAddr> {
    let mut protos = input.iter();
    if !matches!(protos.next().map(|p| p.code()), Some(DnsAddr::CODE | Ip4::CODE | Ip6::CODE)) {
        return None;
    }
    if !matches!(protos.next().map(|p| p.code()), Some(Tcp::CODE)) {
        return None;
    }
    if let Some(p) = protos.next() {
        if let Some(p) = p.cast::<Service>() {
            if &*p == "api" {
                return MultiAddr::default().try_with(input.iter().take(3)).ok();
            }
        }
    }
    None
}

/// Create a secure channel to an API node or return the address route as is.
async fn connect(
    manager: &mut NodeManager,
    addr: &MultiAddr,
    auth: &HashMap<MultiAddr, IdentityIdentifier>,
    mode: CredentialExchangeMode,
) -> Result<Either<Address, Route>> {
    if let Some(a) = api_host(addr) {
        if let Some(i) = auth.get(&a) {
            let r = multiaddr_to_route(&a).ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
            let i = Some(vec![i.clone()]);
            let a = manager.create_secure_channel_impl(r, i, mode, None).await?;
            return Ok(Either::Left(a));
        }
    }
    let r = multiaddr_to_route(addr).ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
    Ok(Either::Right(r))
}

/// Configure the session for automatic recovery.
fn enable_recovery(
    session: &mut Session,
    manager: Address,
    ctx: Arc<Context>,
    addr: MultiAddr,
    alias: String,
    auth: Arc<HashMap<MultiAddr, IdentityIdentifier>>,
    mode: CredentialExchangeMode,
) {
    session.set_replacement(move |prev| {
        let ctx = ctx.clone();
        let addr = addr.clone();
        let alias = alias.clone();
        let auth = auth.clone();
        let manager = manager.clone();
        Box::pin(async move {
            debug!(%addr, "creating new remote forwarder");
            let f = async {
                let a = replace_sec_chan(&ctx, &manager, prev, &addr, &auth, mode).await?;
                RemoteForwarder::create_static(&ctx, Route::from(a.clone()), alias).await?;
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
    prev: Address,
    addr: &MultiAddr,
    authorised: &HashMap<MultiAddr, IdentityIdentifier>,
    mode: CredentialExchangeMode,
) -> Result<Address> {
    debug!(%addr, %prev, "recreating secure channel");
    let req = DeleteSecureChannelRequest::new(&prev);
    let req = Request::delete("/node/secure_channel").body(req).to_vec()?;
    let vec: Vec<u8> = ctx.send_and_receive(manager.clone(), req).await?;
    let mut d = Decoder::new(&vec);
    let res: Response = d.decode()?;
    if res.status() != Some(Status::Ok) && res.has_body() {
        let e: Error = d.decode()?;
        debug!(%addr, %prev, err = ?e.message(), "failed to delete secure channel");
    }
    let ids = authorised.get(addr).map(|i| vec![i.clone()]);
    let mut req = CreateSecureChannelRequest::new(addr, ids, mode);
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
    let mad = res.addr()?;
    if let Some(p) = mad.first() {
        if let Some(p) = p.cast::<Service>() {
            return Ok(Address::from_string(&*p));
        }
    }
    Err(ApiError::generic("invalid response address"))
}
