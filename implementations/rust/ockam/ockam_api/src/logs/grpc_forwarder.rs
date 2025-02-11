use crate::logs::secure_client_service::OckamGrpcRequest;
use crate::{ApiError, Result};
use hyper::{http, Uri};
use ockam_core::api::{Method, Request};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Routed, Worker};
use ockam_node::Context;
use std::future;
use tonic::body::BoxBody;
use tonic::client::GrpcService;
use tonic::transport::Channel;

/// The GrpcForwarder worker accepts gRPC requests serialized as Ockam messages
/// and forwards them to an HTTP endpoint.
///
/// Note that we don't wait for a response from the endpoint.
pub struct GrpcForwarder {
    channel: Channel,
}

impl GrpcForwarder {
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
impl Worker for GrpcForwarder {
    type Message = Request<OckamGrpcRequest>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _ctx: &mut Context,
        message: Routed<Request<OckamGrpcRequest>>,
    ) -> ockam_core::Result<()> {
        let request = message.into_body()?;
        let (header, body) = request.into_parts();
        if let (Some(Method::Post), "/", Some(ockam_request)) =
            (header.method(), header.path(), body)
        {
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
