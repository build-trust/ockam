use crate::credentials::credentials_retriever::CredentialsRetriever;
use crate::{
    Credential, Credentials, IdentitiesReader, Identity, IdentityError, IdentityIdentifier,
};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;

/// An AuthorityService represents an authority which issued credentials
#[derive(Clone)]
pub struct AuthorityService {
    identities_reader: Arc<dyn IdentitiesReader>,
    credentials: Arc<dyn Credentials>,
    identifier: IdentityIdentifier,
    own_credential: Option<Arc<dyn CredentialsRetriever>>,
}

impl AuthorityService {
    /// Create a new authority service
    pub fn new(
        identities_reader: Arc<dyn IdentitiesReader>,
        credentials: Arc<dyn Credentials>,
        identifier: IdentityIdentifier,
        own_credential: Option<Arc<dyn CredentialsRetriever>>,
    ) -> Self {
        Self {
            identities_reader,
            credentials,
            identifier,
            own_credential,
        }
    }

    /// Return the Public Identity of the Authority
    pub async fn identity(&self) -> Result<Identity> {
        self.identities_reader.get_identity(&self.identifier).await
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
