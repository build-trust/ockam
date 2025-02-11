use crate::ApiError;
use crate::Result;
use futures_core::future::BoxFuture;
use http_body_util::{BodyExt, Full};
use hyper::{http, Method, Version};
use minicbor::{Decode, Decoder, Encode};
use ockam::identity::SecureClient;
use ockam_core::api::Request;
use ockam_core::{Decodable, Encodable, Encoded, TryClone};
use ockam_node::Context;
use std::str::FromStr;
use std::task::Poll;
use tonic::body::BoxBody;
use tonic::codegen::Service;

/// This service implements the `tower_service::Service` trait
/// to post http Requests to an Ockam node as Ockam messages, via a secure channel defined by the
/// SecureClient.
///
/// The Context is an Option since because it is necessary to clone this struct and attempting
/// to attempt a Context could fail if the current node is being shut down.
///
pub struct SecureClientService {
    secure_client: SecureClient,
    ctx: Option<Context>,
    service_address: String,
}

impl Clone for SecureClientService {
    fn clone(&self) -> Self {
        let ctx_clone = if let Some(ctx) = &self.ctx {
            ctx.try_clone().ok()
        } else {
            None
        };

        SecureClientService {
            secure_client: self.secure_client.clone(),
            ctx: ctx_clone,
            service_address: self.service_address.clone(),
        }
    }
}

impl SecureClientService {
    /// Create a new SecureClientService from:
    /// - A SecureClient in order to create a secure channel to another node
    /// - A Context to be able to send messages to the node
    /// - A service_address to route messages to the correct service
    pub fn new(
        secure_client: SecureClient,
        ctx: &Context,
        service_address: &str,
    ) -> SecureClientService {
        SecureClientService {
            secure_client,
            ctx: ctx.try_clone().ok(),
            service_address: service_address.to_string(),
        }
    }
}

/// Implementation the tower_service::Service interface to send http Requests with a
/// binary payload to an Ockam service located in another node
impl Service<http::Request<BoxBody>> for SecureClientService {
    type Response = http::Response<BoxBody>;
    type Error = ApiError;
    type Future = BoxFuture<'static, Result<Self::Response>>;

    /// This function checks if the service is ready to accept requests.
    /// For now we consider that this is always the case.
    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: http::Request<BoxBody>) -> Self::Future {
        let mut service = self.clone();
        Box::pin(async move { service.send_request(request).await })
    }
}

impl SecureClientService {
    /// Send an http Request to the Ockam node via a secure channel:
    /// - Transform the http Request into an Ockam message
    /// - Send the message to the node
    /// - Don't wait for a response from the node and return a successful one.
    async fn send_request(
        &mut self,
        request: http::Request<BoxBody>,
    ) -> Result<http::Response<BoxBody>> {
        if let Some(ctx) = &self.ctx {
            let ockam_request_body = Self::make_ockam_request_body(request).await?;
            let _ = self
                .secure_client
                .tell(
                    ctx,
                    &self.service_address,
                    Request::post("/").body(ockam_request_body),
                )
                .await?;
        };
        http::Response::builder()
            .body(BoxBody::default())
            .map_err(ApiError::message)
    }

    /// In order to make an Ockam request we collect all the bytes from the http request body.
    /// We could improve this by chunking the original body into several Ockam messages if the original body is too big.
    async fn make_ockam_request_body(request: http::Request<BoxBody>) -> Result<OckamRequest> {
        let mut bytes: Vec<u8> = Vec::new();
        let (head, mut body) = request.into_parts();
        while let Some(frame) = body.frame().await {
            if let Ok(f) = frame {
                if let Some(chunk) = f.data_ref() {
                    bytes.extend_from_slice(chunk);
                }
            }
        }
        Ok(OckamRequest::from(http::Request::from_parts(head, bytes)))
    }
}

/// This struct represent an http Request sent as an Encodable Ockam message
///
/// The from and to_http_request methods are used to convert between the two.
///
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct OckamRequest {
    #[n(0)]
    method: String,
    #[n(1)]
    uri: String,
    #[n(2)]
    version: HttpVersion,
    #[n(3)]
    headers: Vec<(String, String)>,
    #[n(4)]
    body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
enum HttpVersion {
    #[n(0)]
    Http09,
    #[n(1)]
    Http10,
    #[n(2)]
    Http11,
    #[n(3)]
    Http2,
    #[n(4)]
    Http3,
}

impl From<http::Request<Vec<u8>>> for OckamRequest {
    fn from(req: http::Request<Vec<u8>>) -> Self {
        Self {
            method: req.method().to_string(),
            uri: req.uri().to_string(),
            version: match req.version() {
                Version::HTTP_09 => HttpVersion::Http09,
                Version::HTTP_10 => HttpVersion::Http10,
                Version::HTTP_11 => HttpVersion::Http11,
                Version::HTTP_2 => HttpVersion::Http2,
                Version::HTTP_3 => HttpVersion::Http3,
                _ => HttpVersion::Http3,
            },
            headers: req
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_string()))
                .collect(),
            body: req.into_body(),
        }
    }
}

impl OckamRequest {
    pub fn make_http_request(self) -> Result<http::Request<BoxBody>> {
        let mut req = http::Request::builder();
        req = req.method(Method::from_str(&self.method).map_err(ApiError::message)?);
        let version = match self.version {
            HttpVersion::Http09 => Version::HTTP_09,
            HttpVersion::Http10 => Version::HTTP_10,
            HttpVersion::Http11 => Version::HTTP_11,
            HttpVersion::Http2 => Version::HTTP_2,
            HttpVersion::Http3 => Version::HTTP_3,
        };
        req = req.version(version);
        req = req.uri(self.uri);
        for (k, v) in self.headers.iter() {
            req = req.header(k.to_string(), v.to_string());
        }
        let body = Full::new(bytes::Bytes::from(self.body))
            .map_err(|never| match never {})
            .boxed_unsync();
        req.body(body).map_err(ApiError::message)
    }
}

impl Decodable for OckamRequest {
    fn decode(bytes: &[u8]) -> ockam_core::Result<Self> {
        let mut decoder = Decoder::new(bytes);
        Ok(decoder.decode()?)
    }
}

impl Encodable for OckamRequest {
    fn encode(self) -> ockam_core::Result<Encoded> {
        Ok(minicbor::to_vec(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::Decodable;

    #[test]
    fn test_make_ockam_request_body() {
        let request = make_http_request();
        let ockam_request_body = OckamRequest::from(request);
        assert_eq!(
            <OckamRequest as Decodable>::decode(
                minicbor::to_vec(ockam_request_body.clone())
                    .unwrap()
                    .as_slice()
            )
            .unwrap(),
            ockam_request_body
        );
    }

    /// HELPERS
    fn make_http_request() -> http::Request<Vec<u8>> {
        http::Request::builder()
            .method(Method::GET)
            .uri("http://localhost:8080/")
            .body("hello".as_bytes().to_vec())
            .unwrap()
    }
}
