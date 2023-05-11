use crate::credentials::credentials_retriever::CredentialsRetriever;
use crate::{Credential, Credentials, Identity, IdentityError, IdentityIdentifier};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;

/// An AuthorityService represents an authority which issued credentials
#[derive(Clone)]
pub struct AuthorityService {
    credentials: Arc<dyn Credentials>,
    identity: Identity,
    own_credential: Option<Arc<dyn CredentialsRetriever>>,
}

impl AuthorityService {
    /// Create a new authority service
    pub fn new(
        credentials: Arc<dyn Credentials>,
        identity: Identity,
        own_credential: Option<Arc<dyn CredentialsRetriever>>,
    ) -> Self {
        Self {
            credentials,
            identity,
            own_credential,
        }
    }

    /// Return the Public Identity of the Authority
    pub fn identity(&self) -> Identity {
        self.identity.clone()
    }

    /// Retrieve the credential for an identity within this authority
    pub async fn credential(
        &self,
        ctx: &Context,
        for_identity: &IdentityIdentifier,
    ) -> Result<Credential> {
        let retriever = self
            .own_credential
            .clone()
            .ok_or(IdentityError::UnknownAuthority)?;
        let credential = retriever.retrieve(ctx, for_identity).await?;

        self.credentials
            .verify_credential(for_identity, &[self.identity().await?], credential.clone())
            .await?;
        Ok(credential)
    }
}
