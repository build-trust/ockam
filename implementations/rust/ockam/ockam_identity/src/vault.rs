use ockam_core::compat::sync::Arc;
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};
use ockam_vault::{
    KeyId, SecureChannelVault, SigningVault, SoftwareSecureChannelVault, SoftwareSigningVault,
    SoftwareVerifyingVault, StoredSecret, VerifyingVault,
};

/// Storage for Vault persistent values
pub type VaultStorage = Arc<dyn KeyValueStorage<KeyId, StoredSecret>>;

/// Vault
#[derive(Clone)]
pub struct Vault {
    /// Vault used for Identity Keys
    pub identity_vault: Arc<dyn SigningVault>,
    /// Vault used for Secure Channels
    pub secure_channel_vault: Arc<dyn SecureChannelVault>,
    /// Vault used for signing Credentials
    pub credential_vault: Arc<dyn SigningVault>,
    /// Vault used for verifying signature and sha256
    pub verifying_vault: Arc<dyn VerifyingVault>,
}

impl Vault {
    /// Constructor
    pub fn new(
        identity_vault: Arc<dyn SigningVault>,
        secure_channel_vault: Arc<dyn SecureChannelVault>,
        credential_vault: Arc<dyn SigningVault>,
        verifying_vault: Arc<dyn VerifyingVault>,
    ) -> Self {
        Self {
            identity_vault,
            secure_channel_vault,
            credential_vault,
            verifying_vault,
        }
    }

    /// Create Software implementation Vault with [`InMemoryKeyVaultStorage`]
    pub fn create() -> Self {
        Self::new(
            Self::create_identity_vault(),
            Self::create_secure_channel_vault(),
            Self::create_credential_vault(),
            Self::create_verifying_vault(),
        )
    }

    /// Create [`SoftwareSigningVault`] with [`InMemoryKeyVaultStorage`]
    pub fn create_identity_vault() -> Arc<dyn SigningVault> {
        Arc::new(SoftwareSigningVault::new(InMemoryKeyValueStorage::create()))
    }

    /// Create [`SoftwareSecureChannelVault`] with [`InMemoryKeyVaultStorage`]
    pub fn create_secure_channel_vault() -> Arc<dyn SecureChannelVault> {
        Arc::new(SoftwareSecureChannelVault::new(
            InMemoryKeyValueStorage::create(),
        ))
    }

    /// Create [`SoftwareSigningVault`] with [`InMemoryKeyVaultStorage`]
    pub fn create_credential_vault() -> Arc<dyn SigningVault> {
        Arc::new(SoftwareSigningVault::new(InMemoryKeyValueStorage::create()))
    }

    /// Create [`SoftwareVerifyingVault`]
    pub fn create_verifying_vault() -> Arc<dyn VerifyingVault> {
        Arc::new(SoftwareVerifyingVault {})
    }
}

impl Vault {
    /// Create Software Vaults with [`PersistentStorage`] with a given path
    pub async fn create_with_persistent_storage_path(
        path: &std::path::Path,
    ) -> ockam_core::Result<Vault> {
        let storage = ockam_vault::storage::PersistentStorage::create(path).await?;
        Ok(Self::create_with_persistent_storage(storage))
    }

    /// Create Software Vaults with a given [`VaultStorage`]r
    pub fn create_with_persistent_storage(storage: VaultStorage) -> Vault {
        Self::new(
            Arc::new(SoftwareSigningVault::new(storage.clone())),
            Arc::new(SoftwareSecureChannelVault::new(storage.clone())),
            Arc::new(SoftwareSigningVault::new(storage)),
            Arc::new(SoftwareVerifyingVault {}),
        )
    }
}
