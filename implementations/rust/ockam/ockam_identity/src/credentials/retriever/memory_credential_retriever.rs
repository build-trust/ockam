use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Result};

use crate::models::CredentialAndPurposeKey;
use crate::{CredentialRetriever, Identifier};

/// Credentials retriever that retrieves a credential from memory
pub struct MemoryCredentialRetriever {
    credential: CredentialAndPurposeKey,
}

impl MemoryCredentialRetriever {
    /// Create a new MemoryCredentialRetriever
    pub fn new(credential: CredentialAndPurposeKey) -> Self {
        Self { credential }
    }
}

#[async_trait]
impl CredentialRetriever for MemoryCredentialRetriever {
    async fn retrieve(&self, _subject: &Identifier) -> Result<CredentialAndPurposeKey> {
        Ok(self.credential.clone())
    }
}
