use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::trace;

use minicbor::{Decode, Encode};

use ockam_core::api::{Error, Response};
use ockam_core::{self, async_trait, AsyncTryClone, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};

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
        send_message: SendMessage,
    ) -> Result<Response<Vec<u8>>, Response<Error>> {
        let multiaddr = send_message.multiaddr()?;
        let msg = send_message.message.to_vec();

        let res = self
            .node_manager
            .send_message(ctx, &multiaddr, msg, None)
            .await;
        match res {
            Ok(r) => Ok(Response::ok().body(r)),
            Err(err) => {
                error!(target: TARGET, ?err, "Failed to send message");
                Err(Response::internal_error_no_request(
                    "Failed to send message",
                ))
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
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>> {
        let msg_length = message.len();
        let connection_ctx = Arc::new(ctx.async_try_clone().await?);
        let connection = self
            .make_connection(connection_ctx, addr, self.identifier(), None, timeout)
            .await?;
        let route = connection.route(self.tcp_transport()).await?;

        trace!(target: TARGET, route = %route, msg_l = %msg_length, "sending message");
        let options = if let Some(timeout) = timeout {
            MessageSendReceiveOptions::new().with_timeout(timeout)
        } else {
            MessageSendReceiveOptions::new()
        };
        Ok(ctx
            .send_and_receive_extended::<Vec<u8>>(route, message, options)
            .await?
            .body())
    }
}

#[async_trait]
pub trait MessageSender {
    async fn send_message(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        message: Vec<u8>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>>;
}
