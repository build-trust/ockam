use crate::local_multiaddr_to_route;
use crate::nodes::models::flow_controls::AddConsumer;
use minicbor::Decoder;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::Result;
use ockam_node::Context;

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) fn add_consumer(
        &self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let request: AddConsumer = dec.decode()?;

        let mut route = match local_multiaddr_to_route(request.address()) {
            None => return Ok(Response::bad_request(req.id())),
            Some(r) => r,
        };
        let addr = route.step()?;
        if !route.is_empty() {
            return Ok(Response::bad_request(req.id()));
        }

        ctx.flow_controls()
            .add_consumer(addr, request.flow_control_id(), request.policy());

        Ok(Response::ok(req.id()))
    }
}
