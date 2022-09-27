use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::multiaddr_to_addr;
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse, CredentialExchangeMode,
    DeleteSecureChannelRequest,
};
use minicbor::Decoder;
use ockam::{Address, Context, Result};
use ockam_core::api::{Error, Request, Response, Status};
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Project, Secure, Tcp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use std::collections::BTreeMap;
use std::time::Duration;

pub(crate) const MAX_RECOVERY_TIME: Duration = Duration::from_secs(10);
const MAX_CONNECT_TIME: Duration = Duration::from_secs(5);

pub(crate) fn resolve_project(
    set: &BTreeMap<String, ProjectLookup>,
    name: &str,
) -> Result<(MultiAddr, IdentityIdentifier)> {
    if let Some(info) = set.get(name) {
        Ok((info.node_route.clone(), info.identity_id.clone()))
    } else {
        Err(ApiError::message(format!("project {name} not found")))
    }
}

pub(crate) async fn delete_sec_chan(
    ctx: &Context,
    manager: &Address,
    addr: &MultiAddr,
) -> Result<()> {
    debug!(%addr, "removing secure channel");
    let req = {
        let a = multiaddr_to_addr(addr)
            .ok_or_else(|| ApiError::message(format!("could not map to address: {addr}")))?;
        DeleteSecureChannelRequest::new(&a)
    };
    let req = Request::delete("/node/secure_channel").body(req).to_vec()?;
    let vec: Vec<u8> = ctx.send_and_receive(manager.clone(), req).await?;
    let mut d = Decoder::new(&vec);
    let res: Response = d.decode()?;
    if res.status() != Some(Status::Ok) && res.has_body() {
        let e: Error = d.decode()?;
        debug!(%addr, err = ?e.message(), "failed to delete secure channel");
    }
    Ok(())
}

pub(crate) async fn create_sec_chan(
    ctx: &Context,
    manager: &Address,
    addr: &MultiAddr,
    auth: Option<IdentityIdentifier>,
    mode: CredentialExchangeMode,
) -> Result<MultiAddr> {
    let auth = auth.map(|a| vec![a]);
    let mut req = CreateSecureChannelRequest::new(addr, auth, mode);
    req.timeout = Some(MAX_CONNECT_TIME);
    let req = Request::post("/node/secure_channel").body(req).to_vec()?;
    let vec: Vec<u8> = ctx.send_and_receive(manager.clone(), req).await?;
    let mut d = Decoder::new(&vec);
    let res: Response = d.decode()?;
    if res.status() != Some(Status::Ok) {
        if res.has_body() {
            let e: Error = d.decode()?;
            warn!(%addr, err = ?e.message(), "failed to create secure channel");
        }
        return Err(ApiError::generic("error creating secure channel"));
    }
    let res: CreateSecureChannelResponse = d.decode()?;
    res.addr()
}

pub(crate) fn starts_with_host_tcp_secure(addr: &MultiAddr) -> Option<usize> {
    let host_match = Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]);
    if addr.matches(0, &[host_match, Tcp::CODE.into(), Secure::CODE.into()]) {
        Some(3)
    } else {
        None
    }
}

pub(crate) async fn connect(
    ctx: &Context,
    manager: &Address,
    addr: &MultiAddr,
    mode: CredentialExchangeMode,
    auth: Option<IdentityIdentifier>,
    projects: &BTreeMap<String, ProjectLookup>,
) -> Result<(MultiAddr, MultiAddr)> {
    if let Some(p) = addr.first() {
        if p.code() == Project::CODE {
            let p = p
                .cast::<Project>()
                .ok_or_else(|| ApiError::message("invalid project protocol in multiaddr"))?;
            let (a, i) = resolve_project(projects, &p)?;
            debug!(addr = %a, "creating secure channel");
            let w = create_sec_chan(ctx, manager, &a, Some(i), mode).await?;
            let a = MultiAddr::default().try_with(addr.iter().skip(1))?;
            return Ok((w, a));
        }
    }

    if let Some(pos) = starts_with_host_tcp_secure(addr) {
        debug!(%addr, "creating secure channel");
        let (a, b) = addr.split(pos);
        let w = create_sec_chan(ctx, manager, &a, auth, mode).await?;
        return Ok((w, b));
    }

    if Some(Secure::CODE) == addr.last().map(|p| p.code()) {
        debug!(%addr, "creating secure channel");
        let w = create_sec_chan(ctx, manager, addr, auth, mode).await?;
        return Ok((w, MultiAddr::default()));
    }

    Ok((MultiAddr::default(), addr.clone()))
}
