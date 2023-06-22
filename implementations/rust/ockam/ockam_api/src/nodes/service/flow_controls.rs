use crate::local_multiaddr_to_route;
use crate::nodes::models::flow_controls::AddConsumer;
use minicbor::Decoder;
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_core::Result;
use ockam_node::Context;

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) fn add_consumer(
        &self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder, ResponseBuilder<Error>> {
        let request: AddConsumer = dec.decode()?;

        let mut route = match local_multiaddr_to_route(request.address()) {
            None => {
                let err_body = Error::new(req.path())
                    .with_message(format!("Invalid address: {}", request.address()));
                return Err(Response::bad_request(req.id()).body(err_body));
            }
            Some(r) => r,
        };

        let addr = match route.step() {
            Ok(a) => a,
            Err(e) => {
                let err_body = Error::new(req.path())
                    .with_message(format!(
                        "Unable to retrieve address from {}.",
                        request.address(),
                    ))
                    .with_cause(Error::new(req.path()).with_message(e.to_string()));
                return Err(Response::bad_request(req.id()).body(err_body));
            }
        };
        if !route.is_empty() {
            let err_body = Error::new(req.path())
                .with_message(format!("Invalid address: {}", request.address()));
            return Err(Response::bad_request(req.id()).body(err_body));
        }

        ctx.flow_controls()
            .add_consumer(addr, request.flow_control_id());

        Ok(Response::ok(req.id()))
    }
}
