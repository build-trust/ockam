use minicbor::{Decode, Encode};

use ockam_core::CowBytes;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct SendMessage<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8400702>,
    #[n(1)] pub route: MultiAddr,
    #[b(2)] pub message: CowBytes<'a>,
}

impl<'a> SendMessage<'a> {
    pub fn new<S: Into<CowBytes<'a>>>(route: MultiAddr, message: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            route,
            message: message.into(),
        }
    }

    pub fn route(&self) -> Result<Route> {
        crate::multiaddr_to_route(&self.route)
            .ok_or_else(|| ApiError::generic(&format!("Invalid MultiAddr: {}", self.route)))
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::{Request, Response, Status};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::nodes::NodeManager;

    const TARGET: &str = "ockam_api::message";

    impl NodeManager {
        pub(crate) async fn send_message(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_body: super::SendMessage = dec.decode()?;
            let route = req_body.route()?;
            let msg = req_body.message.to_vec();
            let msg_length = msg.len();

            trace!(target: TARGET, route = %req_body.route, msg_l = %msg_length, "sending message");

            let res: Result<Vec<u8>> = ctx.send_and_receive(route, msg).await;
            match res {
                Ok(r) => Ok(Response::builder(req.id(), Status::Ok).body(r).to_vec()?),
                Err(err) => {
                    error!(target: TARGET, ?err, "Failed to send message");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            }
        }
    }
}
