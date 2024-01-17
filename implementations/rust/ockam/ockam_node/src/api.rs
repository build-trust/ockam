#![allow(missing_docs)]

use core::fmt::{Display, Formatter};
use minicbor::{Decode, Encode};
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use ockam_core::api::Reply::Successful;
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{LocalInfo, Result, Route};

use crate::{Context, MessageSendReceiveOptions};

/// This struct provides some support for making requests to another node
/// and receiving replies
pub struct Client {
    route: Route,
    timeout: Option<Duration>,
}

impl Client {
    /// Create a new client to send messages to a given destination
    /// A default timeout can be specified
    /// WARNING: The caller is responsible for cleaning all the resources
    ///          involved in the Route when it's no longer used (like TCP connections or Secure Channels)
    pub fn new(route: &Route, timeout: Option<Duration>) -> Self {
        Self {
            route: route.clone(),
            timeout,
        }
    }

    /// Send a request of type T and receive a reply of type R
    ///
    /// The result is a `Result<Reply<R>>` where `Reply<R>` can contain a value of type `R` but
    /// might be an error and a status code if the request was not successful.
    ///
    /// This allows to distinguish:
    ///
    ///  - communication errors
    ///  - request failures
    ///  - successes
    ///
    /// Note that a `Reply<T>` can be converted in a `Result<T>` by using the `success()?` method
    /// if one is not interested in request failures.
    pub async fn ask<T, R>(&self, ctx: &Context, req: Request<T>) -> Result<Reply<R>>
    where
        T: Encode<()>,
        R: for<'a> Decode<'a, ()>,
    {
        let bytes: Vec<u8> = self.request_with_timeout(ctx, req, self.timeout).await?;
        Response::parse_response_reply::<R>(bytes.as_slice())
    }

    /// Send a request of type T and don't expect a reply
    /// See `ask` for more information
    pub async fn tell<T>(&self, ctx: &Context, req: Request<T>) -> Result<Reply<()>>
    where
        T: Encode<()>,
    {
        let request_header = req.header().clone();
        let bytes = self.request_with_timeout(ctx, req, self.timeout).await?;
        let (response, decoder) = Response::parse_response_header(bytes.as_slice())?;
        if !response.is_ok() {
            Ok(Reply::Failed(
                Error::from_failed_request(&request_header, &response.parse_err_msg(decoder)),
                response.status(),
            ))
        } else {
            Ok(Successful(()))
        }
    }

    /// Send a request of type T and expect an untyped reply
    /// See `ask` for more information
    pub async fn request<T>(&self, ctx: &Context, req: Request<T>) -> Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        self.request_with_timeout(ctx, req, self.timeout).await
    }

    /// Send a request of type T and expect an untyped reply within a specific timeout
    /// See `ask` for more information
    pub async fn request_with_timeout<T>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let (response, _) = self.request_with_local_info(ctx, req, timeout).await?;
        Ok(response)
    }

    /// Send a request of type T and expect an untyped reply within a specific timeout
    /// Additionally provide any local information added to the received message
    /// See `ask` for more information
    pub async fn ask_with_local_info<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Option<Duration>,
    ) -> Result<(Reply<R>, Vec<LocalInfo>)>
    where
        T: Encode<()>,
        R: for<'a> Decode<'a, ()>,
    {
        let (bytes, local_info) = self.request_with_local_info(ctx, req, timeout).await?;
        let reply = Response::parse_response_reply::<R>(bytes.as_slice())?;
        Ok((reply, local_info))
    }

    /// Send a request of type T and expect an untyped reply within a specific timeout
    /// Additionally provide any local information added to the received message
    /// See `ask` for more information
    async fn request_with_local_info<T>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Option<Duration>,
    ) -> Result<(Vec<u8>, Vec<LocalInfo>)>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        let opentelemetry_context = OpenTelemetryContext::current();
        let req = req.tracing_context(opentelemetry_context.to_string());

        req.encode(&mut buf)?;
        trace! {
            target:  "ockam_api",
            id     = %req.header().id(),
            method = ?req.header().method(),
            path   = %req.header().path(),
            body   = %req.header().has_body(),
        };
        let options = if let Some(t) = timeout {
            MessageSendReceiveOptions::new().with_timeout(t)
        } else {
            MessageSendReceiveOptions::new().without_timeout()
        };

        // TODO: Check IdentityId is the same we sent message to?
        // TODO: Check response id matches request id?
        let resp = ctx
            .send_and_receive_extended::<Vec<u8>>(self.route.clone(), buf, options)
            .await?;
        let local_info = resp.local_message().local_info().to_vec();
        let body = resp.body();

        Ok((body, local_info))
    }
}

/// Serializable data type to hold the opentelemetry propagation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenTelemetryContext(HashMap<String, String>);

impl OpenTelemetryContext {
    pub fn extract(&self) -> opentelemetry::Context {
        global::get_text_map_propagator(|propagator| propagator.extract(self))
    }

    fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn inject(context: &opentelemetry::Context) -> Self {
        global::get_text_map_propagator(|propagator| {
            let mut propagation_context = OpenTelemetryContext::empty();
            propagator.inject_context(context, &mut propagation_context);
            propagation_context
        })
    }

    pub fn current() -> OpenTelemetryContext {
        OpenTelemetryContext::inject(&opentelemetry::Context::current())
    }
}

impl Display for OpenTelemetryContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&serde_json::to_string(&self).map_err(|_| core::fmt::Error)?)
    }
}

impl Injector for OpenTelemetryContext {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
}

impl Extractor for OpenTelemetryContext {
    fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_owned();
        self.0.get(&key).map(|v| v.as_ref())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_ref()).collect()
    }
}

/// Parse the OpenTelemetry context from a String
impl TryFrom<&str> for OpenTelemetryContext {
    type Error = ockam_core::Error;

    fn try_from(value: &str) -> Result<Self> {
        opentelemetry_context_parser(value)
    }
}

/// Parse the OpenTelemetry context from a String
impl TryFrom<String> for OpenTelemetryContext {
    type Error = ockam_core::Error;

    fn try_from(value: String) -> Result<Self> {
        opentelemetry_context_parser(&value)
    }
}

/// Parse the OpenTelemetry context from a String
pub fn opentelemetry_context_parser(input: &str) -> Result<OpenTelemetryContext> {
    serde_json::from_str(input).map_err(|e| {
        ockam_core::Error::new(
            Origin::Api,
            Kind::Serialization,
            format!("Invalid OpenTelemetry context: {input}. Got error: {e:?}"),
        )
    })
}
