use std::str::FromStr;
use std::sync::Arc;
use tracing::trace;

use minicbor::Decoder;
use minicbor::{Decode, Encode};

use ockam_core::api::{RequestHeader, Response};
use ockam_core::{self, async_trait, AsyncTryClone, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::error::ApiError;
use crate::nodes::{NodeManager, NodeManagerWorker};

const TARGET: &str = "ockam_api::message";

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct SendMessage {
    #[n(1)] pub route: String,
    #[n(2)] pub message: Vec<u8>,
}

impl SendMessage {
    pub fn new(route: &MultiAddr, message: Vec<u8>) -> Self {
        Self {
            route: route.to_string(),
            message,
        }
    }

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::core(format!("Invalid route: {}", self.route)))
    }
}

impl NodeManagerWorker {
    pub(crate) async fn send_message(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let req_body: SendMessage = dec.decode()?;
        let multiaddr = req_body.multiaddr()?;
        let msg = req_body.message.to_vec();

        let res = self.node_manager.send_message(ctx, &multiaddr, msg).await;
        match res {
            Ok(r) => Ok(Response::ok(req).body(r).to_vec()?),
            Err(err) => {
                error!(target: TARGET, ?err, "Failed to send message");
                Ok(Response::internal_error(req, "Failed to send message").to_vec()?)
            }
        }
    }
}

#[async_trait]
impl MessageSender for NodeManager {
    async fn send_message(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        message: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let msg_length = message.len();
        let connection_ctx = Arc::new(ctx.async_try_clone().await?);
        let connection = self
            .make_connection(connection_ctx, addr, None, None, None, None)
            .await?;
        let route = connection.route()?;

        trace!(target: TARGET, route = %route, msg_l = %msg_length, "sending message");
        ctx.send_and_receive::<Vec<u8>>(route, message).await
    }
}

#[async_trait]
pub trait MessageSender {
    async fn send_message(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        message: Vec<u8>,
    ) -> Result<Vec<u8>>;
}
