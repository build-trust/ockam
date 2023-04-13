use std::str::FromStr;

use minicbor::{Decode, Encode};

use ockam_core::Result;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{CowBytes, CowStr};
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;

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

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::generic(&format!("Invalid route: {}", self.route)))
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use crate::error::ApiError;
    use crate::local_multiaddr_to_route;
    use crate::nodes::connection::Connection;
    use ockam_core::api::{Request, Response, Status};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::nodes::{NodeManager, NodeManagerWorker};

    const TARGET: &str = "ockam_api::message";

    impl NodeManagerWorker {
        pub(crate) async fn send_message(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_body: super::SendMessage = dec.decode()?;
            let multiaddr = req_body.multiaddr()?;
            let msg = req_body.message.to_vec();
            let msg_length = msg.len();

            let connection = Connection::new(ctx, &multiaddr);
            let connection_instance =
                NodeManager::connect(self.node_manager.clone(), connection).await?;

            let route = local_multiaddr_to_route(&connection_instance.normalized_addr)
                .ok_or_else(|| ApiError::generic("Invalid route"))?;

            trace!(target: TARGET, route = %route, msg_l = %msg_length, "sending message");

            let res = ctx.send_and_receive::<Vec<u8>>(route, msg).await;
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
