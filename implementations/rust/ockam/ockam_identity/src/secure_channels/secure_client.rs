use crate::{Identifier, SecureChannelOptions, TrustIdentifierPolicy};
use minicbor::{Decode, Encode};

use crate::{SecureChannel, SecureChannels};
use ockam_core::api::Reply::Successful;
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::{self, route, Result, Route};
use ockam_node::api::Client;
use ockam_node::Context;

/// This client creates a secure channel to a node
/// and can then send a typed request to that node (and receive a typed response)
///
/// Note that a secure channel is created every time a request is made.
/// There is no attempt to keep the channel alive to send other requests
///
/// In order for this client to work:
///
///  - the requested node must have started a transport listener
///  - the requested node must have started a secure channel listener
///  - the `secure_route` must start with the transport address of a worker connected to the requested node transport listener
///  - the requested node must have started the services named `api_service` in the `ask/tell` methods
///
#[derive(Clone)]
pub struct SecureClient {
    // secure_channels is used to create a secure channel before sending a request
    secure_channels: Arc<SecureChannels>,
    // destination for the secure channel
    secure_route: Route,
    // identifier of the secure channel responder
    server_identifier: Identifier,
    // identifier of the secure channel initiator
    client_identifier: Identifier,
    // default timeout to use for receiving a reply
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
    ///  1. first a secure channel is created
    ///  2. then a request is sent to a specific service named `api_service`
    ///  3. when a response is received, it is decoded to the type R
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
    ///
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
        let bytes: Vec<u8> = self
            .request_with_timeout(ctx, api_service, req, self.timeout)
            .await?;
        Response::parse_response_reply::<R>(bytes.as_slice())
    }

    /// Send a request of type T and don't expect a reply
    /// See `ask` for more information
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
    /// See `ask` for more information
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
    /// See `ask` for more information
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
        let client = Client::new(&route, Some(timeout));
        let response = client.request(ctx, req).await?;
        self.secure_channels
            .stop_secure_channel(ctx, sc.encryptor_address())
            .await?;
        Ok(response)
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
