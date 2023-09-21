use minicbor::Decoder;

use ockam_core::api::{Error, RequestHeader, Response};
use ockam_core::Result;
use ockam_node::Context;

use crate::local_multiaddr_to_route;
use crate::nodes::models::flow_controls::AddConsumer;

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) fn add_consumer(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let request: AddConsumer = dec.decode()?;

        let mut route = local_multiaddr_to_route(request.address())?;

        let addr = match route.step() {
            Ok(a) => a,
            Err(e) => {
                return Err(Response::bad_request(
                    req,
                    &format!(
                        "Unable to retrieve address from {}: {}",
                        request.address(),
                        e
                    ),
                ));
            }
        };
        if !route.is_empty() {
            return Err(Response::bad_request(
                req,
                &format!("Invalid address: {}.", request.address(),),
            ));
        }

        ctx.flow_controls()
            .add_consumer(addr, request.flow_control_id());

        Ok(Response::ok(req))
    }
}
