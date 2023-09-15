use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

use ockam_core::api::Request;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, Result, Route};
use ockam_node::{Context, DEFAULT_TIMEOUT};

use crate::{Credential, Identifier, SecureChannels, SecureClient};
use crate::models::CredentialAndPurposeKey;

/// Trait for retrieving a credential for a given identity
#[async_trait]
pub trait CredentialsRetriever: Send + Sync + 'static {
    /// Retrieve a credential for an identity
    async fn retrieve(
        &self,
        ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<CredentialAndPurposeKey>;
}

/// Credentials retriever that retrieves a credential from memory
pub struct CredentialsMemoryRetriever {
    credential_and_purpose_key: CredentialAndPurposeKey,
}

impl CredentialsMemoryRetriever {
    /// Create a new CredentialsMemoryRetriever
    pub fn new(credential_and_purpose_key: CredentialAndPurposeKey) -> Self {
        Self {
            credential_and_purpose_key,
        }
    }
}

#[async_trait]
impl CredentialsRetriever for CredentialsMemoryRetriever {
    /// Retrieve a credential stored in memory
    async fn retrieve(
        &self,
        _ctx: &Context,
        _for_identity: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        Ok(self.credential_and_purpose_key.clone())
    }
}

/// Credentials retriever for credentials located on a different node
pub struct RemoteCredentialsRetriever {
    secure_channels: Arc<SecureChannels>,
    issuer: RemoteCredentialsRetrieverInfo,
}

impl RemoteCredentialsRetriever {
    /// Create a new remote credential retriever
    pub fn new(
        secure_channels: Arc<SecureChannels>,
        issuer: RemoteCredentialsRetrieverInfo,
    ) -> Self {
        Self {
            secure_channels,
            issuer,
        }
    }

    fn make_secure_client(&self, for_identity: &IdentityIdentifier) -> SecureClient {
        SecureClient::new(
            self.secure_channels.clone(),
            self.issuer.route.clone(),
            self.issuer.identifier.clone(),
            for_identity.clone(),
            Duration::from_secs(DEFAULT_TIMEOUT),
        )
    }
}

#[async_trait]
impl CredentialsRetriever for RemoteCredentialsRetriever {
    async fn retrieve(
        &self,
        ctx: &Context,
        for_identity: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        debug!("Getting credential from : {}", &self.issuer.route);
        let client = self.make_secure_client(for_identity);
        let credential = client
            .ask(ctx, "credential_issuer", Request::post("/"))
            .await?
            .success()?;
        Ok(credential)
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialsRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub identifier: Identifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialsRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(identifier: Identifier, route: Route, service_address: Address) -> Self {
        Self {
            identifier,
            route,
            service_address,
        }
    }
}
