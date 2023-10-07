use ockam_core::compat::sync::Arc;
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};
use ockam_vault::legacy::{KeyId, StoredSecret};
use ockam_vault::{
    SoftwareVaultForSecureChannels, SoftwareVaultForSigning, SoftwareVaultForVerifyingSignatures,
    VaultForSecureChannels, VaultForSigning, VaultForVerifyingSignatures,
};

/// Storage for Vault persistent values
pub type VaultStorage = Arc<dyn KeyValueStorage<KeyId, StoredSecret>>;

/// Vault
#[derive(Clone)]
pub struct Vault {
    /// Vault used for Identity Keys
    pub identity_vault: Arc<dyn VaultForSigning>,
    /// Vault used for Secure Channels
    pub secure_channel_vault: Arc<dyn VaultForSecureChannels>,
    /// Vault used for signing Credentials
    pub credential_vault: Arc<dyn VaultForSigning>,
    /// Vault used for verifying signature and sha256
    pub verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
}

impl Vault {
    /// Constructor
    pub fn new(
        identity_vault: Arc<dyn VaultForSigning>,
        secure_channel_vault: Arc<dyn VaultForSecureChannels>,
        credential_vault: Arc<dyn VaultForSigning>,
        verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
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

    /// Create [`SoftwareVaultForSigning`] with [`InMemoryKeyVaultStorage`]
    pub fn create_identity_vault() -> Arc<dyn VaultForSigning> {
        Arc::new(SoftwareVaultForSigning::new(
            InMemoryKeyValueStorage::create(),
        ))
    }

    /// Create [`SoftwareSecureChannelVault`] with [`InMemoryKeyVaultStorage`]
    pub fn create_secure_channel_vault() -> Arc<dyn VaultForSecureChannels> {
        Arc::new(SoftwareVaultForSecureChannels::new(
            InMemoryKeyValueStorage::create(),
        ))
    }

    /// Create [`SoftwareVaultForSigning`] with [`InMemoryKeyVaultStorage`]
    pub fn create_credential_vault() -> Arc<dyn VaultForSigning> {
        Arc::new(SoftwareVaultForSigning::new(
            InMemoryKeyValueStorage::create(),
        ))
    }

    /// Create [`SoftwareVaultForVerifyingSignatures`]
    pub fn create_verifying_vault() -> Arc<dyn VaultForVerifyingSignatures> {
        Arc::new(SoftwareVaultForVerifyingSignatures {})
    }
}

impl Vault {
    /// Create Software Vaults with [`PersistentStorage`] with a given path
    #[cfg(feature = "std")]
    pub async fn create_with_persistent_storage_path(
        path: &std::path::Path,
    ) -> ockam_core::Result<Vault> {
        let storage = ockam_vault::storage::PersistentStorage::create(path).await?;
        Ok(Self::create_with_persistent_storage(storage))
    }

    /// Create Software Vaults with a given [`VaultStorage`]r
    pub fn create_with_persistent_storage(storage: VaultStorage) -> Vault {
        Self::new(
            Arc::new(SoftwareVaultForSigning::new(storage.clone())),
            Arc::new(SoftwareVaultForSecureChannels::new(storage.clone())),
            Arc::new(SoftwareVaultForSigning::new(storage)),
            Arc::new(SoftwareVaultForVerifyingSignatures {}),
        )
    }
}
