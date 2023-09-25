use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_node::Context;
use tracing::{debug, error};

use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::{AuthorityService, IdentityError};

/// A trust context defines which authorities are trusted to attest to which attributes, within a context.
/// Our first implementation assumes that there is only one authority and it is trusted to attest to all attributes within this context.
#[derive(Clone)]
pub struct TrustContext {
    /// This is the ID of the trust context; which is primarily used for ABAC policies
    id: String,
    /// Authority capable of retrieving credentials
    authority: Option<AuthorityService>,
}

impl TrustContext {
    /// Create a new Trust Context
    pub fn new(id: String, authority: Option<AuthorityService>) -> Self {
        Self { id, authority }
    }

    /// Return the ID of the Trust Context
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the Authority of the Trust Context
    pub fn authority(&self) -> Result<&AuthorityService> {
        self.authority
            .as_ref()
            .ok_or_else(|| IdentityError::UnknownAuthority.into())
    }

    /// Return the authority identities attached to this trust context
    pub async fn authorities(&self) -> Result<Vec<Identifier>> {
        Ok(vec![self.authority()?.identifier().clone()])
    }

    /// Return the credential for a given identity if an Authority has been defined
    /// and can issue a credential for that identity
    pub async fn get_credential(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> Option<CredentialAndPurposeKey> {
        match self.authority().ok() {
            Some(authority) => match authority.credential(ctx, identifier).await {
                Ok(credential) => {
                    debug!("retrieved a credential using the trust context authority");
                    Some(credential)
                }
                Err(e) => {
                    error!(
                        "no credential could be retrieved {}, authority {}, subject {}",
                        e.to_string(),
                        authority.identifier(),
                        identifier
                    );
                    None
                }
            },
            None => {
                debug!("no authority is defined on the trust context");
                None
            }
        }
    }
}
