use crate::credentials::credentials_retriever::CredentialsRetriever;
use crate::models::{CredentialAndPurposeKey, Identifier, TimestampInSeconds};
use crate::utils::{add_seconds, now};
use crate::{Credentials, IdentityError};
use tracing::debug;

use ockam_core::compat::sync::Arc;
use ockam_core::compat::sync::RwLock;
use ockam_core::Result;
use ockam_node::Context;

/// An AuthorityService represents an authority which issued credentials
#[derive(Clone)]
pub struct AuthorityService {
    credentials: Arc<Credentials>,
    identifier: Identifier,
    own_credential: Option<Arc<dyn CredentialsRetriever>>,
    inner_cache: Arc<RwLock<Option<CachedCredential>>>,
}

#[derive(Clone)]
struct CachedCredential {
    credential: CredentialAndPurposeKey,
    valid_until: TimestampInSeconds,
}

impl AuthorityService {
    /// Create a new authority service
    pub fn new(
        credentials: Arc<Credentials>,
        identifier: Identifier,
        own_credential: Option<Arc<dyn CredentialsRetriever>>,
    ) -> Self {
        Self {
            credentials,
            identifier,
            own_credential,
            inner_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Retrieve the credential for an identity within this authority
    pub async fn credential(
        &self,
        ctx: &Context,
        subject: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        {
            // check if we have a valid cached credential
            let guard = self.inner_cache.read().unwrap();
            let now = now()?;
            if let Some(cache) = guard.as_ref() {
                // add an extra minute to have a bit of leeway for clock skew
                if cache.valid_until > add_seconds(&now, 60) {
                    return Ok(cache.credential.clone());
                }
            }
        }

        // in order to keep the locking schema simple, we allow multiple concurrent retrievals
        let retriever = self
            .own_credential
            .clone()
            .ok_or(IdentityError::UnknownAuthority)?;
        let credential = retriever.retrieve(ctx, subject).await?;
        debug!("retrieved a credential for subject {}", subject);

        let credential_data = self
            .credentials
            .credentials_verification()
            .verify_credential(Some(subject), &[self.identifier.clone()], &credential)
            .await?;

        debug!("the retrieved credential is valid");

        let mut guard = self.inner_cache.write().unwrap();
        *guard = Some(CachedCredential {
            credential: credential.clone(),
            valid_until: credential_data.credential_data.expires_at,
        });

        Ok(credential)
    }

    /// Issuer [`Identifier`]
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }
}
