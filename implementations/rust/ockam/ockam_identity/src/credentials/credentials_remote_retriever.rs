#![cfg(feature = "std")]
use crate::{
    Credential, CredentialsIssuerClient, CredentialsRetriever, Identity, SecureChannelOptions,
    SecureChannels, TrustMultiIdentifiersPolicy,
};
use core::time::Duration;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, route, Address, Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use tracing::{debug, error};

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
        debug!("Getting credential from : {}", &self.issuer.multiaddr);
        let route = self.issuer.resolve_route(ctx, &self.flow_controls).await?;

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
                route,
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
#[derive(Debug, Clone)]
pub struct RemoteCredentialsRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub identity: Identity,
    /// Multiaddr used to establish a secure channel to the remote node
    pub multiaddr: MultiAddr,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialsRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(identity: Identity, multiaddr: MultiAddr, service_address: Address) -> Self {
        Self {
            identity,
            multiaddr,
            service_address,
        }
    }

    async fn resolve_route(&self, ctx: &Context, flow_controls: &FlowControls) -> Result<Route> {
        let Some(authority_tcp_session) = self.multiaddr.to_route(ctx, flow_controls).await else {
            let err_msg = format!("Invalid route within trust context: {}", &self.multiaddr);
            error!("{err_msg}");
            return Err(ockam_core::Error::new(Origin::Application, Kind::Unknown, err_msg));
        };
        Ok(authority_tcp_session.route)
    }
}
