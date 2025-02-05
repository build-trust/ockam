use crate::logs::secure_client_service::OckamRequest;
use crate::{ApiError, Result};
use hyper::{http, Uri};
use minicbor::Decoder;
use ockam_core::api::{Method, RequestHeader};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Routed, Worker};
use ockam_node::Context;
use std::future;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::Channel;

/// The HttpForwarder worker accepts http requests serialized as Ockam messages
/// and forwards them to an HTTP endpoint.
///
/// Note that we don't wait for a response from the endpoint.
pub struct HttpForwarder {
    channel: Channel,
}

impl HttpForwarder {
    /// Create a Channel for the given URI
    pub async fn new(uri: Uri) -> Result<Self> {
        let channel = Channel::builder(uri.clone())
            .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
            .map_err(|e| {
                ApiError::message(format!(
                    "cannot create a TLS config for the HttpForwarder channel at {uri:?}: {e:?}"
                ))
            })?
            .connect()
            .await
            .map_err(|e| ApiError::message(format!("cannot connect to {uri:?}: {e:?}")))?;

        Ok(Self { channel })
    }

    /// Forward an http Request.
    /// We don't wait for a response here.
    async fn forward_http_request(&mut self, request: http::Request<BoxBody>) -> Result<()> {
        self.ready().await.map_err(ApiError::core)?;
        let _ = self
            .channel
            .call(request)
            .await
            .map_err(ApiError::message)?;

        Ok(())
    }

    /// Check if the channel is ready before making a call
    async fn ready(&mut self) -> Result<()> {
        future::poll_fn(|cx| self.channel.poll_ready(cx))
            .await
            .map_err(ApiError::message)
    }
}

#[async_trait]
impl Worker for HttpForwarder {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _ctx: &mut Context,
        message: Routed<Vec<u8>>,
    ) -> ockam_core::Result<()> {
        let body = message.into_body()?;
        let mut dec = Decoder::new(&body);
        let header: RequestHeader = dec.decode()?;
        if let (Some(Method::Post), "/") = (header.method(), header.path()) {
            let ockam_request: OckamRequest = dec.decode()?;
            // Every posted message must be forwarded
            let http_request = ockam_request.make_http_request().map_err(|e| {
                ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}"))
            })?;
            self.forward_http_request(http_request)
                .await
                .map_err(|e| ockam_core::Error::new(Origin::Api, Kind::Io, format!("{e:?}")))?;
        };
        Ok(())
    }
}
