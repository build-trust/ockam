#![allow(missing_docs)]

use crate::{Context, MessageSendReceiveOptions};
use ockam_core::api::{Reply, Request, Response};
use ockam_core::compat::string::ToString;
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::{LocalInfo, Message, Result, Route};

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
        T: Message,
        R: Message,
    {
        let response: Response<Vec<u8>> = self.request_with_timeout(ctx, req, self.timeout).await?;
        response.to_reply()
    }

    /// Send a request of type T and don't expect a reply
    /// See `ask` for more information
    pub async fn tell<T>(&self, ctx: &Context, req: Request<T>) -> Result<Reply<()>>
    where
        T: Message,
    {
        let request_header = req.header().clone();
        let response: Response<Vec<u8>> = self.request_with_timeout(ctx, req, self.timeout).await?;
        response.to_empty_reply(&request_header)
    }

    /// Send a request of type T and expect an untyped reply
    /// See `ask` for more information
    pub async fn request<T, R>(&self, ctx: &Context, req: Request<T>) -> Result<Response<R>>
    where
        T: Message,
        R: Message,
    {
        self.request_with_timeout(ctx, req, self.timeout).await
    }

    /// Send a request of type T and expect an untyped reply within a specific timeout
    /// See `ask` for more information
    pub async fn request_with_timeout<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Option<Duration>,
    ) -> Result<Response<R>>
    where
        T: Message,
        R: Message,
    {
        let (response, _) = self.request_with_local_info(ctx, req, timeout).await?;
        Ok(response)
    }

    /// Send a request of type T and expect a response within a specific timeout
    /// Additionally provide any local information added to the received message
    /// See `ask` for more information
    async fn request_with_local_info<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Option<Duration>,
    ) -> Result<(Response<R>, Vec<LocalInfo>)>
    where
        T: Message,
        R: Message,
    {
        let request_header = req.header();
        trace! {
            target:  "ockam_api",
            id     = %request_header.id(),
            method = ?request_header.method(),
            path   = %request_header.path(),
            body   = %request_header.has_body(),
            "sending request"
        }
        let options = if let Some(t) = timeout {
            MessageSendReceiveOptions::new().with_timeout(t)
        } else {
            MessageSendReceiveOptions::new().without_timeout()
        };

        // TODO: Check IdentityId is the same we sent message to?
        // TODO: Check response id matches request id?
        let resp = ctx
            .send_and_receive_extended(self.route.clone(), req, options)
            .await?;
        let local_info = resp.local_message().local_info().to_vec();
        let body: Response<R> = resp.into_body()?;
        let response_header = body.header();

        trace! {
            target:  "ockam_api",
            id     = %response_header.id(),
            body   = %response_header.has_body(),
            status = %response_header.status().map(|s| s.to_string()).unwrap_or("no status".to_string()),
            "received response"
        }

        Ok((body, local_info))
    }
}
