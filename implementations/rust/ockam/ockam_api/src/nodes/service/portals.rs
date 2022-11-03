use crate::error::ApiError;
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletList, InletStatus, OutletList, OutletStatus,
};
use crate::nodes::registry::{InletInfo, OutletInfo, Registry};
use crate::nodes::service::random_alias;
use crate::session::{util, Data, Replacer, Session};
use crate::{actions, resources};
use crate::{multiaddr_to_route, try_multiaddr_to_addr};
use minicbor::Decoder;
use ockam::compat::asynchronous::RwLock;
use ockam::compat::tokio::time::timeout;
use ockam::tcp::{InletOptions, OutletOptions};
use ockam::{Address, Result};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{Action, Env, PolicyAccessControl, PolicyStorage, Resource};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::{AccessControl, AllowAll};
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::proto::{Project, Secure, Service};
use ockam_multiaddr::{MultiAddr, Protocol};
use std::sync::Arc;

use super::{NodeManager, NodeManagerWorker};

const INLET_WORKER: &str = "inlet-worker";
const OUTER_CHAN: &str = "outer-chan";

impl NodeManager {
    async fn access_control(
        &self,
        r: &Resource,
        a: &Action,
        project_id: Option<String>,
    ) -> Result<Arc<dyn AccessControl>> {
        if let Some(pid) = project_id {
            // Populate environment with known attributes:
            let mut env = Env::new();
            env.put("resource.id", str(r.as_str()));
            env.put("action.id", str(a.as_str()));
            env.put("resource.project_id", str(pid));
            // Check if a policy exists for (resource, action) and if not, then
            // create a default entry:
            if self.policies.get_policy(r, a).await?.is_none() {
                let fallback = and([
                    eq([ident("resource.project_id"), ident("subject.project_id")]),
                    eq([ident("subject.role"), str("member")]),
                ]);
                self.policies.set_policy(r, a, &fallback).await?
            }
            let store = self.authenticated_storage.clone();
            let policies = self.policies.clone();
            Ok(Arc::new(PolicyAccessControl::new(
                policies,
                store,
                r.clone(),
                a.clone(),
                env,
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
                        info.bind_addr.as_str(),
                        info.worker_addr.to_string(),
                        alias.as_str(),
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
                    OutletStatus::new(
                        info.tcp_addr.as_str(),
                        info.worker_addr.to_string(),
                        alias.as_str(),
                        None,
                    )
                })
                .collect(),
        ))
    }

    pub(super) async fn create_inlet<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<InletStatus<'a>>> {
        let manager = self.node_manager.clone();
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

        // The addressing scheme is very flexible. Typically the node connects to
        // the cloud via secure channel and the with another secure channel via
        // forwarder to the actual outlet on the target node. However it is also
        // possible that there is just a single secure channel used to go directly
        // to another node.
        let (outer, rest) = {
            let (sec1, rest) = node_manager
                .connect(req.outlet_addr(), req.authorized(), None)
                .await?;
            if !sec1.is_empty() && rest.matches(0, &[Service::CODE.into(), Secure::CODE.into()]) {
                let addr = sec1.clone().try_with(rest.iter().take(2))?;
                let (sec2, _) = node_manager.connect(&addr, None, None).await?;
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

        let resource = req.alias().map(Resource::new).unwrap_or(resources::INLET);

        let check_credential = match req.check_credential() {
            Some(b) => b,
            None => node_manager.enable_credential_checks,
        };
        let project_id = if check_credential {
            let pid = req
                .outlet_addr()
                .first()
                .and_then(|p| {
                    if let Some(p) = p.cast::<Project>() {
                        node_manager
                            .projects
                            .get(&*p)
                            .map(|info| info.id.to_string())
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
        };

        let access_control = node_manager
            .access_control(&resource, &actions::HANDLE_MESSAGE, project_id)
            .await?;

        let options = InletOptions::new(
            listen_addr.clone(),
            outlet_route.clone(),
            access_control.clone(),
        );

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
                    let mut s = Session::new(without_outlet_address(rest));
                    s.data().put(INLET_WORKER, worker_addr.clone());
                    s.data().put(OUTER_CHAN, outer);
                    let repl = replacer(
                        manager,
                        s.data(),
                        listen_addr.clone(),
                        req.outlet_addr().clone(),
                        req.authorized(),
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
        let resource = alias
            .as_deref()
            .map(Resource::new)
            .unwrap_or(resources::OUTLET);
        let alias = alias.map(|a| a.0.into()).unwrap_or_else(random_alias);

        info!("Handling request to create outlet portal");
        let worker_addr = Address::from(worker_addr.as_ref());

        let check_credential = match check_credential {
            Some(b) => b,
            None => node_manager.enable_credential_checks,
        };
        let project_id = if check_credential {
            Some(node_manager.project_id()?.to_string())
        } else {
            None
        };

        let access_control = node_manager
            .access_control(&resource, &actions::HANDLE_MESSAGE, project_id)
            .await?;

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

/// Create a session replacer.
///
/// This returns a function that accepts the previous ping address (e.g.
/// the secure channel worker address) and constructs the whole route
/// again.
fn replacer(
    manager: Arc<RwLock<NodeManager>>,
    data: Data,
    bind: String,
    addr: MultiAddr,
    auth: Option<IdentityIdentifier>,
    access: Arc<dyn AccessControl>,
) -> Replacer {
    Box::new(move |prev| {
        let addr = addr.clone();
        let auth = auth.clone();
        let bind = bind.clone();
        let manager = manager.clone();
        let access = access.clone();
        let data = data.clone();
        Box::pin(async move {
            debug!(%prev, %addr, "creating new tcp inlet");
            // The future that recreates the inlet:
            let f = async {
                let prev = try_multiaddr_to_addr(&prev)?;
                let mut this = manager.write().await;
                let timeout = Some(util::MAX_CONNECT_TIME);

                // First the previous secure channel is deleted, and -- if secure
                // channels were nested -- the outer one as well:

                let _ = this.delete_secure_channel(&prev).await;
                if let Some(a) = data.get::<MultiAddr>(OUTER_CHAN) {
                    let a = try_multiaddr_to_addr(&a)?;
                    let _ = this.delete_secure_channel(&a).await;
                }

                // Now a connection attempt is made:

                let rest = {
                    let (sec1, rest) = this.connect(&addr, auth, timeout).await?;
                    if !sec1.is_empty()
                        && rest.matches(0, &[Service::CODE.into(), Secure::CODE.into()])
                    {
                        // Another secure channel needs to be created. The first one
                        // needs to be remembered so it can be cleaned up if this recovery
                        // executes multiple times:
                        data.put(OUTER_CHAN, sec1.clone());

                        let addr = sec1.clone().try_with(rest.iter().take(2))?;
                        let (sec2, _) = this.connect(&addr, None, timeout).await?;
                        sec2.try_with(rest.iter().skip(2))?
                    } else {
                        sec1.try_with(&rest)?
                    }
                };

                let r = multiaddr_to_route(&rest)
                    .ok_or_else(|| ApiError::message(format!("invalid multiaddr: {rest}")))?;

                // The previous inlet worker needs to be stopped:
                if let Some(wa) = data.get::<Address>(INLET_WORKER) {
                    let _ = this.tcp_transport.stop_inlet(wa).await;
                }

                // Finally attempt to create a new inlet using the new route:
                let opts = InletOptions::new(bind, r, access);
                let wa = this.tcp_transport.create_inlet_extended(opts).await?.0;
                data.put(INLET_WORKER, wa);

                Ok(without_outlet_address(rest))
            };

            // The above future is given some limited time to succeed.
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
