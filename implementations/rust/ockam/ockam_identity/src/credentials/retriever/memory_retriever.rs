use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, Result};

use crate::models::CredentialAndPurposeKey;
use crate::{CredentialRetriever, CredentialRetrieverCreator, Identifier};

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
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn retrieve(&self) -> Result<CredentialAndPurposeKey> {
        Ok(self.credential.clone())
    }

    fn subscribe(&self, _address: &Address) -> Result<()> {
        Ok(())
    }

    fn unsubscribe(&self, _address: &Address) -> Result<()> {
        Ok(())
    }
}

/// Creator for [`MemoryCredentialRetriever`]
pub struct MemoryCredentialRetrieverCreator {
    credential: CredentialAndPurposeKey,
}

impl MemoryCredentialRetrieverCreator {
    /// Constructor
    pub fn new(credential: CredentialAndPurposeKey) -> Self {
        Self { credential }
    }
}

#[async_trait]
impl CredentialRetrieverCreator for MemoryCredentialRetrieverCreator {
    async fn create(&self, _subject: &Identifier) -> Result<Arc<dyn CredentialRetriever>> {
        Ok(Arc::new(MemoryCredentialRetriever::new(
            self.credential.clone(),
        )))
    }
}
