use async_trait::async_trait;

use crate::{credential::Credential, error::IdentityError, Identity, PublicIdentity};
use ockam_core::compat::{boxed::Box, string::String, sync::Arc};

/// Trust Context is a set of information about a trusted authority
#[derive(Clone)]
pub struct TrustContext {
    id: String,
    authority: Option<AuthorityInfo>,
}

impl TrustContext {
    /// Create a new Trust Context
    pub fn new(id: String, authority: Option<AuthorityInfo>) -> Self {
        Self { id, authority }
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
#[derive(Clone)]
pub struct AuthorityInfo {
    identity: PublicIdentity,
    own_credential: Option<Arc<dyn CredentialRetriever>>,
}

impl AuthorityInfo {
    /// Create a new Authority Info
    pub fn new(
        identity: PublicIdentity,
        own_credential: Option<Arc<dyn CredentialRetriever>>,
    ) -> Self {
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
    pub fn own_credential(&self) -> Option<&Arc<dyn CredentialRetriever>> {
        self.own_credential.as_ref()
    }

    /// Retrieve the credential for an identity within this authority
    pub async fn credential(
        &self,
        for_identity: &Identity,
    ) -> Result<Credential, ockam_core::Error> {
        let retriever = self
            .own_credential()
            .ok_or(IdentityError::UnknownAuthority)?;
        let credential = retriever.retrieve(self.identity(), for_identity).await?;

        for_identity
            .verify_self_credential(&credential, vec![&self.identity].into_iter())
            .await?;

        Ok(credential)
    }
}

/// Trait for retrieving a credential
#[async_trait]
pub trait CredentialRetriever: Send + Sync + 'static {
    /// Retrieve a credential for an identity
    async fn retrieve(
        &self,
        identity: &PublicIdentity,
        for_identity: &Identity,
    ) -> Result<Credential, ockam_core::Error>;
}

/// Credential retriever that retrieves a credential from memory
pub struct CredentialMemoryRetriever {
    credential: Credential,
}

impl CredentialMemoryRetriever {
    /// Create a new CredentialMemoryRetriever
    pub fn new(credential: Credential) -> Self {
        Self { credential }
    }
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
