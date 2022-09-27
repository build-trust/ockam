use crate::authenticator::direct::{PROJECT_ID, ROLE};
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletList, InletStatus, OutletList, OutletStatus,
};
use crate::nodes::models::secure_channel::CredentialExchangeMode;
use crate::nodes::registry::{InletInfo, OutletInfo, Registry};
use crate::nodes::service::random_alias;
use crate::session::{util, Data, Replacer, Session};
use minicbor::Decoder;
use ockam::compat::tokio::time::timeout;
use ockam::tcp::{InletOptions, OutletOptions};
use ockam::{Address, Context, Result, TcpTransport};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::{AccessControl, AllowAll};
use ockam_identity::credential::access_control::CredentialAccessControl;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::{Project, Secure, Service};
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_vault::ockam_core::AsyncTryClone;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::{NodeManager, NodeManagerWorker};

const INLET_WORKER: &str = "inlet-worker";
const OUTER_CHAN: &str = "outer-chan";

impl NodeManager {
    fn access_control(&self, project_id: Option<Vec<u8>>) -> Result<Arc<dyn AccessControl>> {
        if let Some(pid) = project_id {
            let required_attributes = vec![
                (PROJECT_ID.to_string(), pid),
                (ROLE.to_string(), b"member".to_vec()),
            ];
            Ok(Arc::new(CredentialAccessControl::new(
                &required_attributes,
                self.authenticated_storage.clone(),
            )))
        } else {
            Ok(Arc::new(AllowAll))
        }
    }
}

impl NodeManagerWorker {
    pub(super) fn get_inlets<'a>(
        &self,
        req: &Request<'a>,
        registry: &'a Registry,
    ) -> ResponseBuilder<InletList<'a>> {
        Response::ok(req.id()).body(InletList::new(
            registry
                .inlets
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
        req: &Request<'a>,
        registry: &'a Registry,
    ) -> ResponseBuilder<OutletList<'a>> {
        Response::ok(req.id()).body(OutletList::new(
            registry
                .outlets
                .iter()
                .map(|(alias, info)| {
                    OutletStatus::new(&info.tcp_addr, info.worker_addr.to_string(), alias, None)
                })
                .collect(),
        ))
    }

    pub(super) async fn create_inlet<'a>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<InletStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let rid = req.id();
        let req: CreateInlet = dec.decode()?;

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

        let (outer, rest) = {
            let (sec1, rest) = node_manager
                .connect(
                    req.outlet_addr(),
                    CredentialExchangeMode::Oneway,
                    req.authorized(),
                )
                .await?;
            if !sec1.is_empty() && rest.matches(0, &[Service::CODE.into(), Secure::CODE.into()]) {
                let addr = sec1.clone().try_with(rest.iter().take(2))?;
                let (sec2, _) = node_manager
                    .connect(&addr, CredentialExchangeMode::Mutual, None)
                    .await?;
                (sec1, sec2.try_with(rest.iter().skip(2))?)
            } else {
                (MultiAddr::default(), sec1.try_with(&rest)?)
            }
        };

        let outlet_route = match multiaddr_to_route(&rest) {
            Some(route) => route,
            None => {
                return Ok(Response::bad_request(rid)
                    .body(InletStatus::bad_request("invalid outlet route")))
            }
        };

        let access_control = node_manager.access_control(if req.is_check_credential() {
            let pid = req
                .outlet_addr()
                .first()
                .and_then(|p| {
                    if let Some(p) = p.cast::<Project>() {
                        node_manager.projects
                            .get(&*p)
                            .map(|info| info.id.as_bytes().to_vec())
                    } else {
                        None
                    }
                })
                .or_else(|| node_manager.project_id.clone());
            if pid.is_none() {
                return Err(ApiError::generic("credential check requires project"));
            }
            pid
        } else {
            None
        })?;

        let options = InletOptions::new(listen_addr.clone(), outlet_route.clone(), access_control.clone());

        let res = node_manager
            .tcp_transport
            .create_inlet_extended(options)
            .await;

        Ok(match res {
            Ok((worker_addr, _)) => {
                // TODO: Use better way to store inlets?
                node_manager.registry.inlets.insert(
                    alias.clone(),
                    InletInfo::new(&listen_addr, Some(&worker_addr), &outlet_route),
                );
                if !outer.is_empty() {
                    let this = ctx.address();
                    let ctx = Arc::new(ctx.async_try_clone().await?);
                    let tcp = Arc::new(node_manager.tcp_transport.async_try_clone().await?);
                    let mut s = Session::new(without_outlet_address(rest));
                    s.data().put(INLET_WORKER, worker_addr.clone());
                    s.data().put(OUTER_CHAN, outer);
                    let repl = replacer(
                        s.data(),
                        this,
                        ctx,
                        tcp,
                        listen_addr.clone(),
                        req.outlet_addr().clone(),
                        req.authorized(),
                        node_manager.projects.clone(),
                        access_control.clone(),
                    );
                    s.set_replacer(repl);
                    node_manager.sessions.lock().unwrap().add(s);
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
                node_manager
                    .registry
                    .inlets
                    .insert(alias.clone(), InletInfo::new(&listen_addr, None, &outlet_route));

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

    pub(super) async fn create_outlet<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<OutletStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateOutlet {
            tcp_addr,
            worker_addr,
            alias,
            check_credential,
            ..
        } = dec.decode()?;
        let tcp_addr = tcp_addr.to_string();

        let alias = alias.map(|a| a.0.into()).unwrap_or_else(random_alias);

        info!("Handling request to create outlet portal");
        let worker_addr = Address::from(worker_addr.as_ref());

        let access_control = node_manager.access_control(if check_credential {
            Some(node_manager.project_id()?.clone())
        } else {
            None
        })?;
        let options = OutletOptions::new(worker_addr.clone(), tcp_addr.clone(), access_control);

        let res = node_manager
            .tcp_transport
            .create_outlet_extended(options)
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
}

/// Configure the session for automatic recovery.
#[allow(clippy::too_many_arguments)]
fn replacer(
    data: Data,
    manager: Address,
    ctx: Arc<Context>,
    tcp: Arc<TcpTransport>,
    bind: String,
    addr: MultiAddr,
    auth: Option<IdentityIdentifier>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
    access: Arc<dyn AccessControl>,
) -> Replacer {
    Box::new(move |prev| {
        let ctx = ctx.clone();
        let addr = addr.clone();
        let tcp = tcp.clone();
        let auth = auth.clone();
        let bind = bind.clone();
        let manager = manager.clone();
        let access = access.clone();
        let data = data.clone();
        let projects = projects.clone();
        Box::pin(async move {
            debug!(%prev, %addr, "creating new tcp inlet");
            let f = async {
                util::delete_sec_chan(&ctx, &manager, &prev).await?;
                if let Some(a) = data.get::<MultiAddr>(OUTER_CHAN) {
                    util::delete_sec_chan(&ctx, &manager, &a).await?;
                }
                let rest = {
                    let (sec1, rest) = {
                        let mode = CredentialExchangeMode::Oneway;
                        util::connect(&ctx, &manager, &addr, mode, auth, &projects).await?
                    };
                    if !sec1.is_empty()
                        && rest.matches(0, &[Service::CODE.into(), Secure::CODE.into()])
                    {
                        data.put(OUTER_CHAN, sec1.clone());
                        let addr = sec1.clone().try_with(rest.iter().take(2))?;
                        let mode = CredentialExchangeMode::Mutual;
                        let (sec2, _) =
                            util::connect(&ctx, &manager, &addr, mode, None, &projects).await?;
                        sec2.try_with(rest.iter().skip(2))?
                    } else {
                        sec1.try_with(&rest)?
                    }
                };
                let r = multiaddr_to_route(&rest)
                    .ok_or_else(|| ApiError::message(format!("invalid multiaddr: {rest}")))?;
                if let Some(wa) = data.get::<Address>(INLET_WORKER) {
                    tcp.stop_inlet(wa).await?
                }
                let opts = InletOptions::new(bind, r, access);
                let wa = tcp.create_inlet_extended(opts).await?.0;
                data.put(INLET_WORKER, wa);
                Ok(without_outlet_address(rest))
            };
            match timeout(util::MAX_RECOVERY_TIME, f).await {
                Err(_) => {
                    warn!(%addr, "timeout creating new tcp inlet");
                    Err(ApiError::generic("timeout"))
                }
                Ok(Err(e)) => {
                    warn!(%addr, err = %e, "error creating new tcp inlet");
                    Err(e)
                }
                Ok(Ok(a)) => Ok(a),
            }
        })
    })
}

fn without_outlet_address(mut addr: MultiAddr) -> MultiAddr {
    if let Some(p) = addr.last() {
        if let Some(a) = p.cast::<Service>() {
            if "outlet" == &*a {
                addr.pop_back();
            }
        }
    }
    addr
}
