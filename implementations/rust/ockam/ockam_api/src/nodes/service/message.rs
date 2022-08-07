use minicbor::{Decode, Encode};
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;

use crate::error::ApiError;
#[cfg(feature = "tag")]
use crate::TypeTag;
use crate::{CowBytes, CowStr};

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct SendMessage<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8400702>,
    #[b(1)] pub route: CowStr<'a>,
    #[b(2)] pub message: CowBytes<'a>,
}

impl<'a> SendMessage<'a> {
    pub fn new<S: Into<CowBytes<'a>>>(route: &MultiAddr, message: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            route: route.to_string().into(),
            message: message.into(),
        }
    }

    pub fn route(&self) -> Result<Route> {
        let maddr = MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::generic(&format!("Invalid route: {}", self.route)))?;
        crate::multiaddr_to_route(&maddr)
            .ok_or_else(|| ApiError::generic(&format!("Invalid MultiAddr: {}", maddr)))
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::nodes::NodeManager;
    use crate::{Request, Response, Status};

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
