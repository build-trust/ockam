use miette::IntoDiagnostic;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::trace;

use minicbor::{CborLen, Decode, Encode};

use ockam_core::api::{Error, Request, Response};
use ockam_core::{self, async_trait, AsyncTryClone, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};

use crate::error::ApiError;
use crate::nodes::{BackgroundNodeClient, NodeManager, NodeManagerWorker};

const TARGET: &str = "ockam_api::message";

#[async_trait]
pub trait Messages {
    async fn send_message(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        message: Vec<u8>,
        timeout: Option<Duration>,
    ) -> miette::Result<Vec<u8>>;
}

#[async_trait]
impl Messages for NodeManager {
    #[instrument(skip_all)]
    async fn send_message(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        message: Vec<u8>,
        timeout: Option<Duration>,
    ) -> miette::Result<Vec<u8>> {
        let msg_length = message.len();
        let connection_ctx = Arc::new(ctx.async_try_clone().await.into_diagnostic()?);
        let connection = self
            .make_connection(connection_ctx, to, self.identifier(), None, timeout)
            .await
            .into_diagnostic()?;
        let route = connection.route().into_diagnostic()?;

        trace!(route = %route, msg_l = %msg_length, "sending message");
        let options = if let Some(timeout) = timeout {
            MessageSendReceiveOptions::new().with_timeout(timeout)
        } else {
            MessageSendReceiveOptions::new()
        };
        Ok(ctx
            .send_and_receive_extended::<Vec<u8>>(route, message, options)
            .await
            .into_diagnostic()?
            .into_body()
            .into_diagnostic()?)
    }
}

#[async_trait]
impl Messages for BackgroundNodeClient {
    #[instrument(skip_all)]
    async fn send_message(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        message: Vec<u8>,
        timeout: Option<Duration>,
    ) -> miette::Result<Vec<u8>> {
        let request = Request::post("v0/message").body(SendMessage::new(to, message));
        Ok(self.clone().set_timeout(timeout).ask(ctx, request).await?)
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

#[derive(Encode, Decode, CborLen, Debug)]
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
