use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::Context;

use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::AuthorityService;

/// A trust context defines which authorities are trusted to attest to which attributes, within a context.
/// Our first implementation assumes that there is only one authority and it is trusted to attest to all attributes within this context.
#[derive(Clone)]
pub struct TrustContext {
    /// This is the ID of the trust context; which is primarily used for ABAC policies
    id: String,

    /// Authority service
    authority_service: Option<AuthorityService>,
}

impl TrustContext {
    /// Create a new Trust Context
    pub fn new(id: String, authority_service: Option<AuthorityService>) -> Self {
        Self {
            id,
            authority_service,
        }
    }

    /// Return the ID of the Trust Context
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the authority identities attached to this trust context
    /// There is only the possibility to have 1 at the moment
    pub fn authorities(&self) -> Vec<Identifier> {
        match self.authority_identifier() {
            Some(identifier) => vec![identifier],
            None => vec![],
        }
    }

    /// Return the authority identifier
    pub fn authority_identifier(&self) -> Option<Identifier> {
        self.authority_service()
            .ok()
            .map(|a| a.identifier().clone())
    }

    /// Return the credential for a given identity if an Authority has been defined
    /// and can issue a credential for that identity
    pub async fn get_credential(
        &self,
        ctx: &Context,
        identifier: &Identifier,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        match self.authority_service().ok() {
            Some(authority_service) => {
                Ok(Some(authority_service.credential(ctx, identifier).await?))
            }
            None => Ok(None),
        }
    }

    /// Return the authority service
    fn authority_service(&self) -> Result<AuthorityService> {
        self.authority_service.clone().ok_or_else(|| {
            Error::new(
                Origin::Identity,
                Kind::Internal,
                format!(
                    "no authority service has been defined for the trust context {}",
                    self.id
                ),
            )
        })
    }
}
