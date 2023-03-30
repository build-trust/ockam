use crate::credential::Credential;
use crate::{Identity, PublicIdentity};
use ockam_core::compat::sync::Arc;

/// Trust Context an entity is part of
pub struct TrustContext {
    /// Id of the trust context
    pub id: String,

    /// Authority information, if any
    pub authority: Option<Arc<AuthorityInfo>>,
}

impl TrustContext {
    /// Builds a new TrustContext struct
    pub fn new(id: String, authority: Option<AuthorityInfo>) -> Self {
        Self {
            id,
            authority: authority.map(Arc::new),
        }
    }
}

/// Trait that all credential retrieval methods must implement
#[ockam_core::async_trait]
pub trait CredentialRetriever: Send + Sync {
    /// Retrieve credential for the given identity
    async fn credential(&self, identity: &Identity) -> Result<Credential, ockam_core::Error>;
}

/// fixed value credential retriever
#[derive(Clone)]
pub struct FromMemoryCredentialRetriever {
    // TODO:  maybe a map from Identity -> Credential?
    cred: Credential,
}

impl FromMemoryCredentialRetriever {
    /// Builds a new FromMemoryCredentialRetriever
    pub fn new(cred: Credential) -> Self {
        Self { cred }
    }
}
#[ockam_core::async_trait]
impl CredentialRetriever for FromMemoryCredentialRetriever {
    async fn credential(&self, _identity: &Identity) -> Result<Credential, ockam_core::Error> {
        Ok(self.cred.clone())
    }
}

/// Information about authority
pub struct AuthorityInfo {
    /// Authority identity
    pub identity: PublicIdentity,

    /// How to obtain a credential issued by this authority.  Optional, there might be no
    /// way, in which case the node can receive (and verify) credentials from other nodes, but can't
    /// present it's own credential back.
    pub credential_retriever: Option<Box<dyn CredentialRetriever>>,
}

impl AuthorityInfo {
    /// Builds a new AuthorityInfo struct
    pub fn new(
        identity: PublicIdentity,
        credential_retriever: Option<Box<dyn CredentialRetriever>>,
    ) -> Self {
        Self {
            identity,
            credential_retriever,
        }
    }
}
