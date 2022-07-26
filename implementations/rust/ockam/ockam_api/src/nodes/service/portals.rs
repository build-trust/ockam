use crate::nodes::models::portal::{CreatePortal, PortalList, PortalStatus, PortalType};
use crate::nodes::service::random_alias;
use crate::nodes::NodeMan;
use crate::{Request, Response, ResponseBuilder};
use minicbor::Decoder;
use ockam::{Address, Result, Route};

impl NodeMan {
    pub(super) fn get_portals(&self, req: &Request<'_>) -> ResponseBuilder<PortalList<'_>> {
        Response::ok(req.id()).body(PortalList::new(
            self.portals
                .iter()
                .map(|((alias, tt), (addr, route))| {
                    PortalStatus::new(
                        *tt,
                        addr,
                        alias,
                        route.as_ref().map(|r| r.to_string().into()),
                    )
                })
                .collect(),
        ))
    }

    pub(super) async fn create_portal<'a>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<PortalStatus<'a>>> {
        let CreatePortal {
            addr,
            alias,
            peer: fwd,
            tt,
            ..
        } = dec.decode()?;
        let addr = addr.to_string();
        let alias = alias.map(|a| a.into()).unwrap_or_else(random_alias);

        let res = match tt {
            PortalType::Inlet => {
                info!("Handling request to create inlet portal");
                let fwd = match fwd {
                    Some(f) => f,
                    None => {
                        return Ok(Response::bad_request(req.id())
                            .body(PortalStatus::bad_request(tt, "invalid request payload")))
                    }
                };

                let outlet_route = match Route::parse(fwd) {
                    Some(route) => route,
                    None => {
                        return Ok(Response::bad_request(req.id())
                            .body(PortalStatus::bad_request(tt, "invalid forward route")))
                    }
                };

                self.tcp_transport
                    .create_inlet(addr.clone(), outlet_route)
                    .await
                    .map(|(addr, _)| addr)
            }
            PortalType::Outlet => {
                info!("Handling request to create outlet portal");
                let self_addr = Address::random_local();
                self.tcp_transport
                    .create_outlet(self_addr.clone(), addr.clone())
                    .await
                    .map(|_| self_addr)
            }
        };

        Ok(match res {
            Ok(addr) => {
                Response::ok(req.id()).body(PortalStatus::new(tt, addr.to_string(), alias, None))
            }
            Err(e) => Response::bad_request(req.id()).body(PortalStatus::new(
                tt,
                addr,
                alias,
                Some(e.to_string().into()),
            )),
        })
    }
}
