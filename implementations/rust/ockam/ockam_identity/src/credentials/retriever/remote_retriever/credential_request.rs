use crate::models::CredentialAndPurposeKey;
use crate::{CredentialsCache, Identifier, IssuerClient};
use ockam_core::compat::sync::Arc;
use ockam_node::compat::asynchronous::Mutex;

/// Retriever that is specific to a given pair issuer/subject
/// in order to serialize calls to the issuer in a concurrent setting
pub struct CredentialRequest {
    issuer: Identifier,
    subject: Identifier,
    issuer_client: Arc<IssuerClient>,
    credentials_cache: Arc<CredentialsCache>,
    mutex: Mutex<()>,
}

impl CredentialRequest {
    /// Create a new retriever specific to a pair issuer / subject
    pub fn new(
        issuer: &Identifier,
        subject: &Identifier,
        credential_issuer_client: Arc<IssuerClient>,
        credentials_cache: Arc<CredentialsCache>,
    ) -> CredentialRequest {
        CredentialRequest {
            issuer: issuer.clone(),
            subject: subject.clone(),
            issuer_client: credential_issuer_client,
            credentials_cache,
            mutex: Mutex::new(()),
        }
    }

    /// Get a new credential and store it locally
    pub async fn run(&self) -> ockam_core::Result<CredentialAndPurposeKey> {
        // make sure that there is only one retrieval at the time
        let _guard = self.mutex.lock().await;
        let credential_and_purpose_key = self.issuer_client.get_credential(&self.subject).await?;
        self.credentials_cache
            .store(
                &self.issuer,
                &self.subject,
                credential_and_purpose_key.get_expires_at()?,
                credential_and_purpose_key.clone(),
            )
            .await?;
        Ok(credential_and_purpose_key)
    }

    /// Return the issuer
    pub fn issuer(&self) -> Identifier {
        self.issuer.clone()
    }

    /// Return the subject
    pub fn subject(&self) -> Identifier {
        self.subject.clone()
    }
}
