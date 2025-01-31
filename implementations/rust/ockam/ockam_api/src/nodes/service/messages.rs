use crate::error::ApiError;
use crate::nodes::{BackgroundNodeClient, NodeManager, NodeManagerWorker};
use miette::IntoDiagnostic;
use minicbor::encode::Write;
use minicbor::{encode, CborLen, Decode, Decoder, Encode, Encoder};
use ockam_core::api::{Error, Request, Response};
use ockam_core::{self, async_trait, Decodable, Encodable, Encoded, Message, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};
use std::str::FromStr;
use std::time::Duration;

const TARGET: &str = "ockam_api::message";

#[async_trait]
pub trait Messages {
    async fn send_message<T: Message, R: Message>(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        message: T,
        timeout: Option<Duration>,
    ) -> miette::Result<R>;
}

#[async_trait]
impl Messages for NodeManager {
    #[instrument(skip_all)]
    async fn send_message<T: Message, R: Message>(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        message: T,
        timeout: Option<Duration>,
    ) -> miette::Result<R> {
        let connection = self
            .make_connection(ctx, to, self.identifier(), None, timeout)
            .await
            .into_diagnostic()?;
        let route = connection.route().into_diagnostic()?;

        trace!(route = %route, "sending message");
        let options = if let Some(timeout) = timeout {
            MessageSendReceiveOptions::new().with_timeout(timeout)
        } else {
            MessageSendReceiveOptions::new()
        };
        Ok(ctx
            .send_and_receive_extended(route, message, options)
            .await
            .into_diagnostic()?
            .into_body()
            .into_diagnostic()?)
    }
}

#[async_trait]
impl Messages for BackgroundNodeClient {
    #[instrument(skip_all)]
    async fn send_message<T: Message, R: Message>(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        message: T,
        timeout: Option<Duration>,
    ) -> miette::Result<R> {
        let request = Request::post("v0/message").body(SendMessage::new(to, message));
        Ok(self.clone().set_timeout(timeout).ask(ctx, request).await?)
    }
}

impl NodeManagerWorker {
    pub(crate) async fn send_message<T: Message, R: Message>(
        &self,
        ctx: &Context,
        send_message: SendMessage<T>,
    ) -> Result<Response<R>, Response<Error>> {
        let multiaddr = send_message.multiaddr()?;
        let msg = send_message.message;

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

#[derive(Debug, Clone, Encode, Decode, CborLen, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SendMessage<T: Message> {
    #[n(1)] pub route: String,
    #[n(2)] pub message: T,
}

impl<T: Message> SendMessage<T> {
    fn encode_send_message<W>(self, buf: W) -> Result<(), encode::Error<W::Error>>
    where
        W: Write,
    {
        let mut e = Encoder::new(buf);
        e.encode(&self.route)?;
        e.writer_mut()
            .write_all(&<T as Encodable>::encode(self.message).map_err(encode::Error::message)?)
            .map_err(|_| encode::Error::message("encoding error"))?;
        Ok(())
    }

    fn into_vec(self) -> Result<Vec<u8>, encode::Error<<Vec<u8> as Write>::Error>> {
        let mut buf = Vec::new();
        self.encode_send_message(&mut buf)?;
        Ok(buf)
    }
}

impl<T: Message> Encodable for SendMessage<T> {
    fn encode(self) -> Result<Encoded> {
        Ok(self.into_vec()?)
    }
}

impl<T: Message> Decodable for SendMessage<T> {
    fn decode(e: &[u8]) -> Result<Self> {
        let mut dec = Decoder::new(e);
        let route: String = dec.decode()?;
        let message = dec.input().get(dec.position()..e.len()).unwrap();
        Ok(SendMessage {
            route,
            message: <T as Decodable>::decode(message)?,
        })
    }
}

impl<T: Message> SendMessage<T> {
    pub fn new(route: &MultiAddr, message: T) -> Self {
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
