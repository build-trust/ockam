use crate::{Credential, Identity};
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Result};
use ockam_node::Context;

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
