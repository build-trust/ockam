use async_trait::async_trait;

use serde::{Deserialize, Serialize};

use crate::{credential::Credential, error::IdentityError, Identity, PublicIdentity};
use ockam_core::compat::{boxed::Box, string::String};

/// Trust Context is a set of information about a trusted authority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustContext {
    id: String,
    authority: Option<AuthorityInfo>,
}

impl TrustContext {
    /// Create a new Trust Context
    pub fn new(authority: AuthorityInfo, id: String) -> Self {
        Self {
            id,
            authority: Some(authority),
        }
    }
    /// Return the ID of the Trust Context
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the Authority of the Trust Context
    pub fn authority(&self) -> Result<&AuthorityInfo, ockam_core::Error> {
        self.authority
            .as_ref()
            .ok_or_else(|| IdentityError::UnknownAuthority.into())
    }
}

/// Authority Info is a set of information defining an authority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityInfo {
    identity: PublicIdentity,
    own_credential: Option<CredentialRetrieverType>,
}

impl AuthorityInfo {
    /// Create a new Authority Info
    pub fn new(identity: PublicIdentity, own_credential: Option<CredentialRetrieverType>) -> Self {
        Self {
            identity,
            own_credential,
        }
    }

    /// Create a new Authority Info without a credential
    pub fn new_identity(identity: PublicIdentity) -> Self {
        Self {
            identity,
            own_credential: None,
        }
    }

    /// Return the Public Identity of the Authority
    pub fn identity(&self) -> &PublicIdentity {
        &self.identity
    }

    /// Return the type of credential retriever for the Authority
    pub fn own_credential(&self) -> Option<&CredentialRetrieverType> {
        self.own_credential.as_ref()
    }

    /// Retrieve the credential for an identity within this authority
    pub async fn credential(
        &self,
        for_identity: &Identity,
        retriever: &impl CredentialRetriever,
    ) -> Result<Credential, ockam_core::Error> {
        let credential = retriever.retrieve(self.identity(), for_identity).await?;

        for_identity
            .verify_self_credential(&credential, vec![&self.identity].into_iter())
            .await?;

        Ok(credential)
    }
}

/// Trait for retrieving a credential
#[async_trait]
pub trait CredentialRetriever {
    /// Retrieve a credential for an identity
    async fn retrieve(
        &self,
        identity: &PublicIdentity,
        for_identity: &Identity,
    ) -> Result<Credential, ockam_core::Error>;
}

/// Type of credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialRetrieverType {
    /// Credential is stored in memory
    FromMemory(Credential),
    /// Path to credential file
    FromPath(String),
    /// MultiAddr to Credential Issuer
    FromCredentialIssuer(String),
}

/// Credential retriever that retrieves a credential from memory
pub struct CredentialMemoryRetriever {
    credential: Credential,
}

#[async_trait]
impl CredentialRetriever for CredentialMemoryRetriever {
    /// Retrieve a credential stored in memory
    async fn retrieve(
        &self,
        _identity: &PublicIdentity,
        _for_identity: &Identity,
    ) -> Result<Credential, ockam_core::Error> {
        Ok(self.credential.clone())
    }
}
