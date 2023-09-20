use minicbor::{Decode, Encode};
use std::sync::Arc;
use std::time::Duration;

use crate::{Identifier, SecureChannelOptions, TrustIdentifierPolicy};
use crate::{SecureChannel, SecureChannels};
use ockam_core::api::Reply::Successful;
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::{self, route, Result, Route};
use ockam_node::api::request_with_options;
use ockam_node::{Context, MessageSendReceiveOptions};

/// This client can create a secure channel to a node
/// and send request / responses
#[derive(Clone)]
pub struct SecureClient {
    pub(crate) secure_channels: Arc<SecureChannels>,
    secure_route: Route,
    server_identifier: Identifier,
    client_identifier: Identifier,
    timeout: Duration,
}

impl SecureClient {
    /// Create a new secure client
    pub fn new(
        secure_channels: Arc<SecureChannels>,
        server_route: Route,
        server_identifier: &Identifier,
        client_identifier: &Identifier,
        timeout: Duration,
    ) -> SecureClient {
        let secure_route = route![server_route.clone()];
        Self {
            secure_channels,
            secure_route,
            server_identifier: server_identifier.clone(),
            client_identifier: client_identifier.clone(),
            timeout,
        }
    }
}

impl SecureClient {
    /// Send a request of type T and receive a reply of type R
    pub async fn ask<T, R>(
        &self,
        ctx: &Context,
        api_service: &str,
        req: Request<T>,
    ) -> Result<Reply<R>>
    where
        T: Encode<()>,
        R: for<'a> Decode<'a, ()>,
    {
        let bytes = self
            .request_with_timeout(ctx, api_service, req, self.timeout)
            .await?;
        Response::parse_response_reply::<R>(&bytes)
    }

    /// Send a request of type T and don't expect a reply
    pub async fn tell<T>(
        &self,
        ctx: &Context,
        api_service: &str,
        req: Request<T>,
    ) -> Result<Reply<()>>
    where
        T: Encode<()>,
    {
        let request_header = req.header().clone();
        let bytes = self
            .request_with_timeout(ctx, api_service, req, self.timeout)
            .await?;
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
    pub async fn request<T>(
        &self,
        ctx: &Context,
        api_service: &str,
        req: Request<T>,
    ) -> Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        self.request_with_timeout(ctx, api_service, req, self.timeout)
            .await
    }

    /// Send a request of type T and expect an untyped reply within a specific timeout
    pub async fn request_with_timeout<T>(
        &self,
        ctx: &Context,
        api_service: &str,
        req: Request<T>,
        timeout: Duration,
    ) -> Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let sc = self.create_secure_channel(ctx).await?;
        let route = route![sc.clone(), api_service];
        let options = MessageSendReceiveOptions::new().with_timeout(timeout);
        let res = request_with_options(ctx, route, req, options).await;
        self.secure_channels
            .stop_secure_channel(ctx, sc.encryptor_address())
            .await?;
        res
    }

    /// Create a secure channel to the node
    pub async fn create_secure_channel(&self, ctx: &Context) -> Result<SecureChannel> {
        let options = SecureChannelOptions::new()
            .with_trust_policy(TrustIdentifierPolicy::new(self.server_identifier.clone()));
        self.secure_channels
            .create_secure_channel(
                ctx,
                &self.client_identifier,
                self.secure_route.clone(),
                options,
            )
            .await
    }

    /// Check if a secure channel can be created to the node
    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        let sc = self.create_secure_channel(ctx).await?;
        self.secure_channels
            .stop_secure_channel(ctx, sc.encryptor_address())
            .await
    }
}
