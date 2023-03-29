use std::time::Duration;

use ockam::route;

use ockam_identity::{
    credential::Credential, Identity, PublicIdentity, SecureChannelTrustOptions,
    TrustMultiIdentifiersPolicy,
};
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::TcpTransport;
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::{
    authenticator::direct::{CredentialIssuerClient, RpcClient},
    cli_state::CredentialState,
    create_tcp_session,
    error::ApiError,
    DefaultAddress,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustContext {
    id: String,
    authority: Option<AuthorityInfo>,
}

impl TrustContext {
    pub fn new(authority: AuthorityInfo) -> Self {
        Self {
            id: hex::encode(&random::<[u8; 4]>()),
            authority: Some(authority),
        }
    }

    pub fn new_with_id(authority: AuthorityInfo, id: String) -> Self {
        Self {
            id,
            authority: Some(authority),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn authority(&self) -> Result<&AuthorityInfo, ockam_core::Error> {
        self.authority
            .as_ref()
            .ok_or_else(|| ApiError::generic("Authority dose not exist."))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityInfo {
    identity: PublicIdentity,
    own_credential: Option<CredentialRetriever>,
}

impl AuthorityInfo {
    pub fn new(identity: PublicIdentity, own_credential: Option<CredentialRetriever>) -> Self {
        Self {
            identity,
            own_credential,
        }
    }

    pub fn new_identity(identity: PublicIdentity) -> Self {
        Self {
            identity,
            own_credential: None,
        }
    }

    pub fn identity(&self) -> &PublicIdentity {
        &self.identity
    }

    pub fn own_credential(&self) -> Option<&CredentialRetriever> {
        self.own_credential.as_ref()
    }

    pub async fn credential(
        &self,
        for_identity: &Identity,
        transport: &TcpTransport,
    ) -> Result<Credential, ockam_core::Error> {
        let cred_retriever = &self
            .own_credential
            .as_ref()
            .ok_or_else(|| ApiError::generic("Credential Retriever was not specified."))?;

        let credential = cred_retriever
            .retrieve(self.identity(), for_identity, transport)
            .await?;

        for_identity
            .verify_self_credential(&credential, vec![&self.identity].into_iter())
            .await?;

        Ok(credential)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialRetriever {
    FromMemory(Credential),
    FromState(CredentialState),
    FromCredentialIssuer(CredentialIssuerInfo),
}

impl CredentialRetriever {
    async fn retrieve(
        &self,
        identity: &PublicIdentity,
        for_identity: &Identity,
        transport: &TcpTransport,
    ) -> Result<Credential, ockam_core::Error> {
        match self {
            CredentialRetriever::FromMemory(v) => Ok(v.clone()),
            CredentialRetriever::FromState(state) => Ok(state.config().await?.credential()?),
            CredentialRetriever::FromCredentialIssuer(issuer) => {
                self.from_credential_issuer(identity, issuer, for_identity, transport)
                    .await
            }
        }
    }

    async fn from_credential_issuer(
        &self,
        identity: &PublicIdentity,
        cred_issuer: &CredentialIssuerInfo,
        for_identity: &Identity,
        transport: &TcpTransport,
    ) -> Result<Credential, ockam_core::Error> {
        // attempt to get credential from authority address
        debug!("Getting credential from : {}", cred_issuer.addr);

        let allowed = vec![identity.identifier().clone()];

        let authority_tcp_session = match create_tcp_session(&cred_issuer.addr, transport).await {
            Some(authority_tcp_session) => authority_tcp_session,
            None => {
                let err_msg = format!("Invalid route within trust context: {}", &cred_issuer.addr);
                error!("{err_msg}");
                return Err(ApiError::generic(&err_msg));
            }
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
    addr: MultiAddr,
}

impl CredentialIssuerInfo {
    pub fn new(addr: MultiAddr) -> Self {
        Self { addr }
    }
}
