use std::time::Duration;

use ockam::{route, TcpTransport};
use ockam_core::async_trait;
use ockam_identity::{
    credential::Credential, CredentialRetriever, Identity, PublicIdentity,
    SecureChannelTrustOptions, TrustMultiIdentifiersPolicy,
};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;
use serde::{Deserialize, Serialize};

use crate::{
    authenticator::direct::{CredentialIssuerClient, RpcClient},
    cli_state::CredentialState,
    create_tcp_session,
    error::ApiError,
    DefaultAddress,
};

#[derive(Debug, Clone)]
pub struct CredentialStateRetriever {
    state: CredentialState,
}

impl CredentialStateRetriever {
    pub fn new(state: CredentialState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl CredentialRetriever for CredentialStateRetriever {
    async fn retrieve(&self, _for_identity: &Identity) -> Result<Credential, ockam_core::Error> {
        Ok(self.state.config()?.credential()?)
    }
}

pub struct CredentialIssuerRetriever {
    issuer: CredentialIssuerInfo,
    transport: TcpTransport,
}

impl CredentialIssuerRetriever {
    pub fn new(issuer: CredentialIssuerInfo, transport: TcpTransport) -> Self {
        Self { issuer, transport }
    }
}

#[async_trait]
impl CredentialRetriever for CredentialIssuerRetriever {
    async fn retrieve(&self, for_identity: &Identity) -> Result<Credential, ockam_core::Error> {
        debug!("Getting credential from : {}", &self.issuer.maddr);

        let allowed = vec![self.issuer.public_identity().await?.identifier().clone()];

        let Some(authority_tcp_session) = create_tcp_session(&self.issuer.maddr, &self.transport).await else {
            let err_msg = format!("Invalid route within trust context: {}", &self.issuer.maddr);
            error!("{err_msg}");
            return Err(ApiError::generic(&err_msg));
        };

        debug!("Create secure channel to authority");

        let trust_options = SecureChannelTrustOptions::new();

        let trust_options = match authority_tcp_session.session {
            Some((sessions, session_id)) => trust_options.as_consumer(&sessions, &session_id),
            None => trust_options,
        };

        let trust_options =
            trust_options.with_trust_policy(TrustMultiIdentifiersPolicy::new(allowed));

        let sc = for_identity
            .create_secure_channel_extended(
                authority_tcp_session.route,
                trust_options,
                Duration::from_secs(120),
            )
            .await?;

        debug!("Created secure channel to project authority");

        let client = CredentialIssuerClient::new(
            RpcClient::new(
                route![sc, DefaultAddress::CREDENTIAL_ISSUER],
                for_identity.ctx(),
            )
            .await?,
        );

        let credential = client.credential().await?;

        Ok(credential)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialIssuerInfo {
    pub identity: String,
    pub maddr: MultiAddr,
}

impl CredentialIssuerInfo {
    pub fn new(identity: String, maddr: MultiAddr) -> Self {
        Self { identity, maddr }
    }

    pub async fn public_identity(&self) -> Result<PublicIdentity, ockam_core::Error> {
        let a = hex::decode(&self.identity)
            .map_err(|_| ApiError::generic("Invalid project authority"))?;
        let p = PublicIdentity::import(&a, Vault::create()).await?;
        Ok(p)
    }
}
