use crate::multiaddr_to_route;
use crate::nodes::models::portal::{
    CreateInlet, CreateOutlet, InletList, InletStatus, OutletList, OutletStatus,
};
use crate::nodes::registry::{InletInfo, OutletInfo};
use crate::nodes::service::{map_multiaddr_err, random_alias};
use crate::nodes::NodeManager;
use minicbor::Decoder;
use ockam::tcp::{InletOptions, OutletOptions};
use ockam::{Address, Result};
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::AllowAll;
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;
use std::sync::Arc;

impl NodeManager {
    pub(super) fn get_inlets(&self, req: &Request<'_>) -> ResponseBuilder<InletList<'_>> {
        Response::ok(req.id()).body(InletList::new(
            self.registry
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

    pub(super) fn get_outlets(&self, req: &Request<'_>) -> ResponseBuilder<OutletList<'_>> {
        Response::ok(req.id()).body(OutletList::new(
            self.registry
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
        let CreateInlet {
            bind_addr,
            outlet_route,
            alias,
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

        let options = InletOptions::new(
            bind_addr.clone(),
            outlet_route,
            Arc::new(AllowAll), // FIXME
        );

        let res = self.tcp_transport.create_inlet_extended(options).await;

        Ok(match res {
            Ok((worker_addr, _)) => {
                // TODO: Use better way to store inlets?
                self.registry.inlets.insert(
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
                self.registry
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
        let CreateOutlet {
            tcp_addr,
            worker_addr,
            alias,
            ..
        } = dec.decode()?;
        let tcp_addr = tcp_addr.to_string();

        let alias = alias.map(|a| a.0.into()).unwrap_or_else(random_alias);

        info!("Handling request to create outlet portal");
        let worker_addr = Address::from(worker_addr.as_ref());

        let options = OutletOptions::new(worker_addr.clone(), tcp_addr.clone(), Arc::new(AllowAll)); // FIXME

        let res = self.tcp_transport.create_outlet_extended(options).await;

        Ok(match res {
            Ok(_) => {
                // TODO: Use better way to store outlets?
                self.registry.outlets.insert(
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
                self.registry
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
