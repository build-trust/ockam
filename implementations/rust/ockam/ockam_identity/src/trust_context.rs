use async_trait::async_trait;

use ockam_transport_tcp::TcpTransport;
use rand::random;
use serde::{Deserialize, Serialize};

use crate::{credential::Credential, error::IdentityError, Identity, PublicIdentity};

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
            .ok_or_else(|| IdentityError::UnknownAuthority.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityInfo {
    identity: PublicIdentity,
    own_credential: Option<CredentialRetrieverType>,
}

impl AuthorityInfo {
    pub fn new(identity: PublicIdentity, own_credential: Option<CredentialRetrieverType>) -> Self {
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

    pub fn own_credential(&self) -> Option<&CredentialRetrieverType> {
        self.own_credential.as_ref()
    }

    pub async fn credential(
        &self,
        for_identity: &Identity,
        transport: &TcpTransport,
        retriever: &impl CredentialRetriever,
    ) -> Result<Credential, ockam_core::Error> {
        let credential = retriever
            .retrieve(self.identity(), for_identity, transport)
            .await?;

        for_identity
            .verify_self_credential(&credential, vec![&self.identity].into_iter())
            .await?;

        Ok(credential)
    }
}

#[async_trait]
pub trait CredentialRetriever {
    async fn retrieve(
        &self,
        identity: &PublicIdentity,
        for_identity: &Identity,
        transport: &TcpTransport,
    ) -> Result<Credential, ockam_core::Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialRetrieverType {
    // Credential is stored in memory
    FromMemory(Credential),
    // Path to credential file
    FromPath(String),
    // MultiAddr to Credential Issuer
    FromCredentialIssuer(String),
}

pub struct CredentialMemoryRetriever {
    credential: Credential,
}

#[async_trait]
impl CredentialRetriever for CredentialMemoryRetriever {
    async fn retrieve(
        &self,
        _identity: &PublicIdentity,
        _for_identity: &Identity,
        _transport: &TcpTransport,
    ) -> Result<Credential, ockam_core::Error> {
        Ok(self.credential.clone())
    }
}

// match self {
//     CredentialRetriever::FromMemory(v) => Ok(v.clone()),
//     CredentialRetriever::FromState(state) => Ok(state.config().await?.credential()?),
//     CredentialRetriever::FromCredentialIssuer(issuer) => {
//         self.from_credential_issuer(identity, issuer, for_identity, transport)
//             .await
//     }
// }
