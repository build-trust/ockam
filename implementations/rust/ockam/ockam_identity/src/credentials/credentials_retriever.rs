use core::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, route, Address, Result, Route};
use ockam_node::Context;

use crate::{
    Credential, CredentialsIssuerClient, IdentityIdentifier, SecureChannelOptions, SecureChannels,
    TrustMultiIdentifiersPolicy,
};

/// Trait for retrieving a credential for a given identity
#[async_trait]
pub trait CredentialsRetriever: Send + Sync + 'static {
    /// Retrieve a credential for an identity
    async fn retrieve(
        &self,
        ctx: &Context,
        for_identity: &IdentityIdentifier,
    ) -> Result<Credential>;
}

/// Credentials retriever that retrieves a credential from memory
pub struct CredentialsMemoryRetriever {
    credential: Credential,
}

impl CredentialsMemoryRetriever {
    /// Create a new CredentialsMemoryRetriever
    pub fn new(credential: Credential) -> Self {
        Self { credential }
    }
}

#[async_trait]
impl CredentialsRetriever for CredentialsMemoryRetriever {
    /// Retrieve a credential stored in memory
    async fn retrieve(
        &self,
        _ctx: &Context,
        _for_identity: &IdentityIdentifier,
    ) -> Result<Credential> {
        Ok(self.credential.clone())
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
}

#[async_trait]
impl CredentialsRetriever for RemoteCredentialsRetriever {
    async fn retrieve(
        &self,
        ctx: &Context,
        for_identity: &IdentityIdentifier,
    ) -> Result<Credential> {
        debug!("Getting credential from : {}", &self.issuer.route);
        let resolved_route = ctx
            .resolve_transport_route(self.issuer.route.clone())
            .await?;
        trace!(
            "Getting credential from resolved route: {}",
            resolved_route.clone()
        );

        let allowed = vec![self.issuer.identifier.clone()];
        debug!("Create secure channel to authority");

        let flow_control_id = FlowControls::generate_id();
        let options = SecureChannelOptions::as_producer(&flow_control_id)
            .with_trust_policy(TrustMultiIdentifiersPolicy::new(allowed));

        let sc = self
            .secure_channels
            .create_secure_channel_extended(
                ctx,
                for_identity,
                resolved_route.clone(),
                options,
                Duration::from_secs(120),
            )
            .await?;

        debug!("Created secure channel to project authority");

        let client =
            CredentialsIssuerClient::new(route![sc, self.issuer.service_address.clone()], ctx)
                .await?;

        let credential = client.credential().await?;
        Ok(credential)
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialsRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub identifier: IdentityIdentifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialsRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(identifier: IdentityIdentifier, route: Route, service_address: Address) -> Self {
        Self {
            identifier,
            route,
            service_address,
        }
    }
}
