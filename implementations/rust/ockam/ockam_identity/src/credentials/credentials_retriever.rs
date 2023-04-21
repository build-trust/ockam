use crate::{
    Credential, CredentialsIssuerClient, Identity, SecureChannelOptions, SecureChannels,
    TrustMultiIdentifiersPolicy,
};
use core::time::Duration;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, route, Address, Error, Result, Route, TransportType};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

/// Trait for retrieving a credential for a given identity
#[async_trait]
pub trait CredentialsRetriever: Send + Sync + 'static {
    /// Retrieve a credential for an identity
    async fn retrieve(&self, ctx: &Context, for_identity: &Identity) -> Result<Credential>;
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
    async fn retrieve(&self, _ctx: &Context, _for_identity: &Identity) -> Result<Credential> {
        Ok(self.credential.clone())
    }
}

/// Credentials retriever for credentials located on a different node
pub struct RemoteCredentialsRetriever {
    secure_channels: Arc<SecureChannels>,
    issuer: RemoteCredentialsRetrieverInfo,
    flow_controls: FlowControls,
}

impl RemoteCredentialsRetriever {
    /// Create a new remote credential retriever
    pub fn new(
        secure_channels: Arc<SecureChannels>,
        issuer: RemoteCredentialsRetrieverInfo,
        flow_controls: FlowControls,
    ) -> Self {
        Self {
            secure_channels,
            issuer,
            flow_controls,
        }
    }
}

#[async_trait]
impl CredentialsRetriever for RemoteCredentialsRetriever {
    async fn retrieve(&self, ctx: &Context, for_identity: &Identity) -> Result<Credential> {
        if !ctx.is_transport_registered(TransportType::new(1)) {
            return Err(Error::new(
                Origin::Transport,
                Kind::NotFound,
                "the TCP transport is required to retrieve remote credentials",
            ));
        };

        debug!("Getting credential from : {}", &self.issuer.route);
        let resolved_route = ctx
            .resolve_transport_route(&self.flow_controls, self.issuer.route.clone())
            .await?;
        trace!(
            "Getting credential from resolved route: {}",
            resolved_route.clone()
        );

        let allowed = vec![self.issuer.identity.identifier()];
        debug!("Create secure channel to authority");

        let flow_control_id = self.flow_controls.generate_id();
        let options = SecureChannelOptions::as_producer(&self.flow_controls, &flow_control_id)
            .as_consumer(&self.flow_controls)
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
                .await?
                .with_flow_controls(&self.flow_controls);

        let credential = client.credential().await?;
        Ok(credential)
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialsRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub identity: Identity,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialsRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(identity: Identity, route: Route, service_address: Address) -> Self {
        Self {
            identity,
            route,
            service_address,
        }
    }
}
