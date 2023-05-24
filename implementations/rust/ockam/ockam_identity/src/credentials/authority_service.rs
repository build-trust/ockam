use crate::credentials::credentials_retriever::CredentialsRetriever;
use crate::{
    Credential, Credentials, IdentitiesReader, Identity, IdentityError, IdentityIdentifier,
    Timestamp,
};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::sync::RwLock;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::Context;

/// An AuthorityService represents an authority which issued credentials
#[derive(Clone)]
pub struct AuthorityService {
    identities_reader: Arc<dyn IdentitiesReader>,
    credentials: Arc<dyn Credentials>,
    identifier: IdentityIdentifier,
    own_credential: Option<Arc<dyn CredentialsRetriever>>,
    inner_cache: Arc<RwLock<Option<CachedCredential>>>,
}

#[derive(Clone)]
struct CachedCredential {
    credential: Credential,
    valid_until: Timestamp,
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
            inner_cache: Arc::new(RwLock::new(None)),
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
        {
            // check if we have a valid cached credential
            let guard = self.inner_cache.read().unwrap();
            let now = Timestamp::now().ok_or_else(|| {
                Error::new(Origin::Application, Kind::Invalid, "invalid system time")
            })?;
            if let Some(cache) = guard.as_ref() {
                // add an extra minute to have a bit of leeway for clock skew
                if cache.valid_until > now.add_seconds(60) {
                    return Ok(cache.credential.clone());
                }
            }
        }

        // in order to keep the locking schema simple, we allow multiple concurrent retrievals
        let retriever = self
            .own_credential
            .clone()
            .ok_or(IdentityError::UnknownAuthority)?;
        let credential = retriever.retrieve(ctx, for_identity).await?;

        let credential_data = self
            .credentials
            .verify_credential(for_identity, &[self.identity().await?], credential.clone())
            .await?;

        let mut guard = self.inner_cache.write().unwrap();
        *guard = Some(CachedCredential {
            credential: credential.clone(),
            valid_until: credential_data.expires,
        });

        Ok(credential)
    }
}
