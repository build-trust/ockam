use crate::{
    KeyId, SecureChannelVault, SigningVault, SoftwareSecureChannelVault, SoftwareSigningVault,
    SoftwareVerifyingVault, StoredSecret, VerifyingVault,
};
use ockam_core::compat::sync::Arc;
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};

/// Storage for Vault persistent values
pub type VaultStorage = Arc<dyn KeyValueStorage<KeyId, StoredSecret>>;

/// Vault
#[derive(Clone)]
pub struct Vault {
    // TODO: It's possible to have 2 separate instances: 1 for Identity keys and 1 for Purpose Keys
    /// Vault used for signing, e.g. Identity Keys and signing Purpose Keys
    pub signing_vault: Arc<dyn SigningVault>,
    /// Vault used for verifying signature and sha256
    pub verifying_vault: Arc<dyn VerifyingVault>,
    /// Vault used for Secure Channels
    pub secure_channel_vault: Arc<dyn SecureChannelVault>,
}

impl Vault {
    /// Constructor
    pub fn new(
        signing_vault: Arc<dyn SigningVault>,
        verifying_vault: Arc<dyn VerifyingVault>,
        secure_channel_vault: Arc<dyn SecureChannelVault>,
    ) -> Self {
        Self {
            signing_vault,
            verifying_vault,
            secure_channel_vault,
        }
    }

    /// Create Software implementation Vault with [`InMemoryKeyVaultStorage`]
    pub fn create() -> Self {
        Self::new(
            Self::create_signing_vault(),
            Self::create_verifying_vault(),
            Self::create_secure_channel_vault(),
        )
    }

    /// Create [`SoftwareSigningVault`] with [`InMemoryKeyVaultStorage`]
    pub fn create_signing_vault() -> Arc<dyn SigningVault> {
        Arc::new(SoftwareSigningVault::new(InMemoryKeyValueStorage::create()))
    }

    /// Create [`SoftwareVerifyingVault`]
    pub fn create_verifying_vault() -> Arc<dyn VerifyingVault> {
        Arc::new(SoftwareVerifyingVault {})
    }

    /// Create [`SoftwareSecureChannelVault`] with [`InMemoryKeyVaultStorage`]
    pub fn create_secure_channel_vault() -> Arc<dyn SecureChannelVault> {
        Arc::new(SoftwareSecureChannelVault::new(
            InMemoryKeyValueStorage::create(),
        ))
    }
}

impl Vault {
    /// Create Software Vaults with [`PersistentStorage`] with a given path
    #[cfg(feature = "storage")]
    pub async fn create_with_persistent_storage_path(
        path: &std::path::Path,
    ) -> ockam_core::Result<Vault> {
        let storage = crate::storage::PersistentStorage::create(path).await?;
        Ok(Self::new(
            Arc::new(SoftwareSigningVault::new(storage.clone())),
            Arc::new(SoftwareVerifyingVault {}),
            Arc::new(SoftwareSecureChannelVault::new(storage)),
        ))
    }

    /// Create Software Vaults with a given [`VaultStorage`]r
    pub fn create_with_persistent_storage(storage: VaultStorage) -> Vault {
        Self::new(
            Arc::new(SoftwareSigningVault::new(storage.clone())),
            Arc::new(SoftwareVerifyingVault {}),
            Arc::new(SoftwareSecureChannelVault::new(storage)),
        )
    }
}
