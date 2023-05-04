use crate::storage::PersistentStorage;
use crate::vault::secrets_store_impl::VaultSecretsStore;
use crate::{
    AsymmetricVault, Kms, SecretsStore, Signer, SymmetricVault, Vault, VaultKms, VaultStorage,
};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::InMemoryKeyValueStorage;
#[cfg(feature = "std")]
use std::path::Path;

/// Builder for Vaults
pub struct VaultBuilder {
    secrets_store: Arc<dyn SecretsStore>,
    asymmetric_vault: Arc<dyn AsymmetricVault>,
    symmetric_vault: Arc<dyn SymmetricVault>,
    signer: Arc<dyn Signer>,
}

impl VaultBuilder {
    pub(crate) fn new_builder() -> VaultBuilder {
        let kms = VaultKms::create_with_storage(InMemoryKeyValueStorage::create());
        let secrets_store = Arc::new(VaultSecretsStore::new(
            kms.clone(),
            InMemoryKeyValueStorage::create(),
        ));
        let asymmetric_vault = secrets_store.clone();
        let symmetric_vault = secrets_store.clone();
        let signer = secrets_store.clone();
        Self {
            secrets_store,
            asymmetric_vault,
            symmetric_vault,
            signer,
        }
    }

    /// Set a persistent storage as a file storage with a specific path
    #[cfg(feature = "std")]
    pub async fn with_persistent_storage_path(&mut self, path: &Path) -> Result<&mut Self> {
        Ok(self.with_persistent_storage(PersistentStorage::create(path).await?))
    }

    /// Set a persistent storage
    pub fn with_persistent_storage(&mut self, persistent_storage: VaultStorage) -> &mut Self {
        self.with_kms(VaultKms::create_with_storage(persistent_storage))
    }

    /// Set a KMS implementation
    pub fn with_kms(&mut self, kms: Arc<dyn Kms>) -> &mut Self {
        self.with_secrets_store(Arc::new(VaultSecretsStore::new(
            kms.clone(),
            InMemoryKeyValueStorage::create(),
        )))
    }

    /// Set a SecretsStore implementation
    pub fn with_secrets_store(&mut self, secrets_store: Arc<dyn SecretsStore>) -> &mut Self {
        self.secrets_store = secrets_store;
        self
    }

    /// Set an AsymmetricVault implementation
    pub fn with_asymmetric_vault(
        &mut self,
        asymmetric_vault: Arc<dyn AsymmetricVault>,
    ) -> &mut Self {
        self.asymmetric_vault = asymmetric_vault;
        self
    }

    /// Set a SymmetricVault implementation
    pub fn with_symmetric_vault(&mut self, symmetric_vault: Arc<dyn SymmetricVault>) -> &mut Self {
        self.symmetric_vault = symmetric_vault;
        self
    }

    /// Set an Signer implementation
    pub fn with_signer(&mut self, signer: Arc<dyn Signer>) -> &mut Self {
        self.signer = signer;
        self
    }

    /// Create a new Vault
    pub fn make(&self) -> Vault {
        Vault {
            secrets_store: self.secrets_store.clone(),
            asymmetric_vault: self.asymmetric_vault.clone(),
            symmetric_vault: self.symmetric_vault.clone(),
            signer: self.signer.clone(),
        }
    }

    /// Create a new Vault in an Arc reference
    pub fn build(&self) -> Arc<Vault> {
        Arc::new(self.make())
    }
}
