use crate::cli_state::NamedCredential;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::Identity;
use ockam_core::async_trait;
use ockam_core::Result;

/// This repository support the storage of credentials retrieved from the command line
/// A credential is associated with a name and its issuer for later retrieval
#[async_trait]
pub trait CredentialsRepository: Send + Sync + 'static {
    /// Store a CredentialAndPurposeKey under a given name
    /// The issuer of the credential is also stored in order to be able to validate the credential
    /// later on
    async fn store_credential(
        &self,
        name: &str,
        issuer: &Identity,
        credential: CredentialAndPurposeKey,
    ) -> Result<NamedCredential>;

    /// Retrieve a credential given its name
    async fn get_credential(&self, name: &str) -> Result<Option<NamedCredential>>;

    /// Retrieve all the stored credentials
    async fn get_credentials(&self) -> Result<Vec<NamedCredential>>;
}
