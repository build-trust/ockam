use crate::storage::VaultFileStorage;
use crate::{KeyId, VaultEntry};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};
use std::path::Path;

/// Vault implementation that stores secrets in memory and uses software crypto.
///
/// # Examples
/// ```
/// use ockam_vault::{CURVE25519_SECRET_LENGTH_U32, SecretAttributes, SecretPersistence, SecretType, SecretVault, Signer, Verifier, Vault};
/// use ockam_core::Result;
///
/// async fn example() -> Result<()> {
///     let mut vault: Vault = Vault::default();
///
///     let mut attributes = SecretAttributes::new(
///         SecretType::X25519,
///         SecretPersistence::Ephemeral,
///         CURVE25519_SECRET_LENGTH_U32,
///     );
///
///     let secret = vault.secret_generate(attributes).await?;
///     let public = vault.secret_public_key_get(&secret).await?;
///
///     let data = "Very important stuff".as_bytes();
///
///     let signature = vault.sign(&secret, data).await?;
///     assert!(vault.verify(&signature, &public, data).await?);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Vault {
    /// Storage for persistent secrets, they are retrieved when the vault
    /// is created again
    pub(crate) persistent_storage: VaultStorage,
    /// Storage for ephemeral secrets, they are lost when the vault is recreated
    pub(crate) ephemeral_storage: InMemoryKeyValueStorage<KeyId, VaultEntry>,
}

impl Default for Vault {
    fn default() -> Self {
        Vault {
            persistent_storage: Arc::new(InMemoryKeyValueStorage::default()),
            ephemeral_storage: InMemoryKeyValueStorage::default(),
        }
    }
}

/// Type alias for the storage of persistent secrets
pub type VaultStorage = Arc<dyn KeyValueStorage<KeyId, VaultEntry>>;

impl Vault {
    /// Create a new Vault
    pub fn new(storage: VaultStorage) -> Self {
        Self {
            persistent_storage: storage,
            ephemeral_storage: InMemoryKeyValueStorage::create(),
        }
    }

    /// Create a new vault with an in memory storage
    pub fn new_in_memory() -> Vault {
        Self::new(Arc::new(InMemoryKeyValueStorage::create()))
    }

    /// Create a new vault with an in memory storage, return as an Arc
    pub fn create() -> Arc<Vault> {
        Arc::new(Self::new_in_memory())
    }

    /// Create a new vault with a persistent storage
    pub async fn create_with_path(path: &Path) -> Result<Arc<Vault>> {
        let vault_storage = VaultFileStorage::create(path).await?;
        Ok(Arc::new(Self::new(vault_storage)))
    }
}
