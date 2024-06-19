use crate::{CredentialRetrieverCreator, Identifier, SecureChannelOptions, TrustIdentifierPolicy};
use minicbor::{Decode, Encode};
use tracing::error;

use crate::{SecureChannel, SecureChannels};
use ockam_core::api::Reply::Successful;
use ockam_core::api::{Error, Reply, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::{self, route, Address, Result, Route};
use ockam_node::api::Client;
use ockam_node::Context;
use ockam_transport_core::Transport;

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
    // Credential retriever
    credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
    // transport to instantiate connections
    transport: Arc<dyn Transport>,
    // destination for the secure channel
    secure_route: Route,
    // identifier of the secure channel responder
    server_identifier: Identifier,
    // identifier of the secure channel initiator
    client_identifier: Identifier,
    // timeout for creating secure channel
    secure_channel_timeout: Duration,
    // default timeout to use for receiving a reply
    request_timeout: Duration,
}

impl SecureClient {
    /// Create a new secure client
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        secure_channels: Arc<SecureChannels>,
        credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
        transport: Arc<dyn Transport>,
        server_route: Route,
        server_identifier: &Identifier,
        client_identifier: &Identifier,
        secure_channel_timeout: Duration,
        request_timeout: Duration,
    ) -> SecureClient {
        Self {
            secure_channels,
            credential_retriever_creator,
            transport,
            secure_route: server_route,
            server_identifier: server_identifier.clone(),
            client_identifier: client_identifier.clone(),
            secure_channel_timeout,
            request_timeout,
        }
    }

    /// Secure Channels
    pub fn secure_channels(&self) -> Arc<SecureChannels> {
        self.secure_channels.clone()
    }

    /// CredentialRetriever
    pub fn credential_retriever_creator(&self) -> Option<Arc<dyn CredentialRetrieverCreator>> {
        self.credential_retriever_creator.clone()
    }

    /// Transport
    pub fn transport(&self) -> Arc<dyn Transport> {
        self.transport.clone()
    }

    /// Route
    pub fn secure_route(&self) -> &Route {
        &self.secure_route
    }

    /// Server Identifier
    pub fn server_identifier(&self) -> &Identifier {
        &self.server_identifier
    }

    /// Client Identifier
    pub fn client_identifier(&self) -> &Identifier {
        &self.client_identifier
    }

    /// Secure Channel cretion timeout
    pub fn secure_channel_timeout(&self) -> Duration {
        self.secure_channel_timeout
    }

    /// Request timeout
    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
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
        match self
            .request_with_timeout(ctx, api_service, req, self.request_timeout)
            .await
        {
            Ok(bytes) => Response::parse_response_reply::<R>(bytes.as_slice()),
            Err(err) => {
                // TODO: we should return a Reply::Failed(timeout) error here
                error!("Error during SecureClient::ask to {} {}", api_service, err);
                Err(err)
            }
        }
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
            .request_with_timeout(ctx, api_service, req, self.request_timeout)
            // TODO: we should return a Reply::Failed(timeout) error here
            .await?;
        let (response, decoder) = Response::parse_response_header(bytes.as_slice())?;
        if response.is_ok() {
            Ok(Successful(()))
        } else {
            Ok(Reply::Failed(
                Error::from_failed_request(&request_header, &response.parse_err_msg(decoder)),
                response.status(),
            ))
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
        self.request_with_timeout(ctx, api_service, req, self.request_timeout)
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
        let (secure_channel, transport_address) = self.create_secure_channel(ctx).await?;
        let route = route![secure_channel.clone(), api_service];
        let client = Client::new(&route, Some(timeout));
        let response = client.request(ctx, req).await;
        let _ = self
            .secure_channels
            .stop_secure_channel(ctx, secure_channel.encryptor_address())
            .await;
        if let Some(transport_address) = transport_address {
            let _ = self.transport.disconnect(transport_address).await;
        }
        // we delay the unwrapping of the response to make sure that the secure channel is
        // properly stopped first
        response
    }

    /// Create a secure channel to the node
    pub async fn create_secure_channel(
        &self,
        ctx: &Context,
    ) -> Result<(SecureChannel, Option<Address>)> {
        let transport_type = self.transport.transport_type();
        let (resolved_route, transport_address) = Context::resolve_transport_route_static(
            self.secure_route.clone(),
            [(transport_type, self.transport.clone())].into(),
        )
        .await?;
        let options = SecureChannelOptions::new()
            .with_trust_policy(TrustIdentifierPolicy::new(self.server_identifier.clone()))
            .with_timeout(self.secure_channel_timeout);

        let options =
            if let Some(credential_retriever_creator) = self.credential_retriever_creator.clone() {
                options.with_credential_retriever_creator(credential_retriever_creator)?
            } else {
                options
            };

        let secure_channel = self
            .secure_channels
            .create_secure_channel(ctx, &self.client_identifier, resolved_route, options)
            .await?;

        Ok((secure_channel, transport_address))
    }

    /// Check if a secure channel can be created to the node
    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        let (secure_channel, transport_address) = self.create_secure_channel(ctx).await?;
        let _ = self
            .secure_channels
            .stop_secure_channel(ctx, secure_channel.encryptor_address())
            .await;
        if let Some(transport_address) = transport_address {
            let _ = self.transport.disconnect(transport_address).await;
        }

        Ok(())
    }
}
