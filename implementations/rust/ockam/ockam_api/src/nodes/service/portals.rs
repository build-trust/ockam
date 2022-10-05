use crate::authenticator::direct::{PROJECT_ID, ROLE};
use crate::multiaddr_to_route;
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletList, InletStatus, OutletList, OutletStatus,
};
use crate::nodes::registry::{InletInfo, OutletInfo, Registry};
use crate::nodes::service::{map_multiaddr_err, random_alias};
use minicbor::Decoder;
use ockam::tcp::{InletOptions, OutletOptions};
use ockam::{Address, Result};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::{AccessControl, AllowAll};
use ockam_identity::credential::access_control::CredentialAccessControl;
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;
use std::sync::Arc;

use super::{NodeManager, NodeManagerWorker};

impl NodeManager {
    fn access_control(&self, check_credential: bool) -> Result<Arc<dyn AccessControl>> {
        if check_credential {
            let project_id = self.project_id()?;
            let required_attributes = vec![
                (PROJECT_ID.to_string(), project_id.clone()),
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
                        // FIXME route.as_ref().map(|r| r.to_string().into()),
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
                        &info.tcp_addr,
                        info.worker_addr.to_string(),
                        alias,
                        None,
                        // FIXME route.as_ref().map(|r| r.to_string().into()),
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
        let mut node_manager = self.node_manager.write().await;
        let CreateInlet {
            bind_addr,
            outlet_route,
            alias,
            check_credential,
            ..
        } = dec.decode()?;
        let bind_addr = bind_addr.to_string();

        let alias = alias.map(|a| a.0.into()).unwrap_or_else(random_alias);

        info!("Handling request to create inlet portal");

        let outlet_route = MultiAddr::from_str(&outlet_route).map_err(map_multiaddr_err)?;
        let outlet_route = match multiaddr_to_route(&outlet_route) {
            Some(route) => route,
            None => {
                return Ok(Response::bad_request(req.id())
                    .body(InletStatus::bad_request("invalid outlet route")))
            }
        };

        let access_control = node_manager.access_control(check_credential)?;
        let options = InletOptions::new(bind_addr.clone(), outlet_route, access_control);

        let res = node_manager
            .tcp_transport
            .create_inlet_extended(options)
            .await;

        Ok(match res {
            Ok((worker_addr, _)) => {
                // TODO: Use better way to store inlets?
                node_manager.registry.inlets.insert(
                    alias.clone(),
                    InletInfo::new(&bind_addr, Some(&worker_addr)),
                );

                Response::ok(req.id()).body(InletStatus::new(
                    bind_addr,
                    worker_addr.to_string(),
                    alias,
                    None,
                ))
            }
            Err(e) => {
                // TODO: Use better way to store inlets?
                node_manager
                    .registry
                    .inlets
                    .insert(alias.clone(), InletInfo::new(&bind_addr, None));

                Response::bad_request(req.id()).body(InletStatus::new(
                    bind_addr,
                    "",
                    alias,
                    Some(e.to_string().into()),
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

        let access_control = node_manager.access_control(check_credential)?;
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
