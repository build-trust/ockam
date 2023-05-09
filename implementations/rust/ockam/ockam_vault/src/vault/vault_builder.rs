#[cfg(feature = "storage")]
use crate::storage::PersistentStorage;
use crate::vault::secrets_store_impl::VaultSecretsStore;
use crate::{
    AsymmetricVault, Implementation, Kms, SecretsStore, Signer, SymmetricVault, Vault, VaultKms,
    VaultStorage,
};
use ockam_core::compat::sync::Arc;
#[cfg(feature = "storage")]
use ockam_core::Result;
use ockam_node::InMemoryKeyValueStorage;
#[cfg(feature = "std")]
use std::path::Path;

/// Builder for Vaults
/// The `VaultBuilder` allows the setting of different implementations for the external interfaces of a Vault:
///   `SecretsStore`, `AsymmetricVault`, `SymmetricVault`, `Signer`.
///
/// It is important to note that the `AsymmetricVault`, `SymmetricVault` and `Signer` interfaces
/// depend on a shared `SecretsStore` implementation for ephemeral and persistent secrets.
/// So when setting specific implementations for these traits it is important that the implementations
/// share consistent storages.
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
    /// Note: this overrides all previously set implementations
    #[cfg(feature = "std")]
    pub async fn with_persistent_storage_path(&mut self, path: &Path) -> Result<&mut Self> {
        Ok(self.with_persistent_storage(PersistentStorage::create(path).await?))
    }

    /// Set a persistent storage
    /// Note: this overrides all previously set implementations
    pub fn with_persistent_storage(&mut self, persistent_storage: VaultStorage) -> &mut Self {
        self.with_kms(VaultKms::create_with_storage(persistent_storage))
    }

    /// Set a KMS implementation
    /// Note: this overrides all previously set implementations
    pub fn with_kms(&mut self, kms: Arc<dyn Kms>) -> &mut Self {
        self.with_secrets_store(VaultSecretsStore::new(
            kms.clone(),
            InMemoryKeyValueStorage::create(),
        ))
    }

    /// Set a SecretsStore implementation
    /// Note: this overrides all previously set implementations
    pub fn with_secrets_store(
        &mut self,
        secrets_store: impl SecretsStore + Clone + Implementation + Kms + 'static,
    ) -> &mut Self {
        self.secrets_store = Arc::new(secrets_store.clone());
        // changing the secrets store resets all other implementations to default ones
        self.asymmetric_vault = Arc::new(secrets_store.clone());
        self.symmetric_vault = Arc::new(secrets_store.clone());
        self.signer = Arc::new(secrets_store);
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
