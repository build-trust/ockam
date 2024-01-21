use ockam_core::compat::sync::Arc;
#[cfg(feature = "storage")]
use ockam_core::Result;
#[cfg(feature = "storage")]
use ockam_node::database::SqlxDatabase;
use ockam_vault::storage::SecretsRepository;
#[cfg(feature = "storage")]
use ockam_vault::storage::SecretsSqlxDatabase;
use ockam_vault::{
    SoftwareVaultForSecureChannels, SoftwareVaultForSigning, SoftwareVaultForVerifyingSignatures,
    VaultForSecureChannels, VaultForSigning, VaultForVerifyingSignatures,
};

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

    /// Create Software implementation Vault with an in-memory storage
    #[cfg(feature = "storage")]
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            Self::create_identity_vault().await?,
            Self::create_secure_channel_vault().await?,
            Self::create_credential_vault().await?,
            Self::create_verifying_vault(),
        ))
    }

    /// Create [`SoftwareVaultForSigning`] with an in-memory storage
    #[cfg(feature = "storage")]
    pub async fn create_identity_vault() -> Result<Arc<dyn VaultForSigning>> {
        Ok(Arc::new(SoftwareVaultForSigning::new(Arc::new(
            SecretsSqlxDatabase::create().await?,
        ))))
    }

    /// Create [`SoftwareSecureChannelVault`] with an in-memory storage
    #[cfg(feature = "storage")]
    pub async fn create_secure_channel_vault() -> Result<Arc<dyn VaultForSecureChannels>> {
        Ok(Arc::new(SoftwareVaultForSecureChannels::new(Arc::new(
            SecretsSqlxDatabase::create().await?,
        ))))
    }

    /// Create [`SoftwareVaultForSigning`] with an in-memory storage
    #[cfg(feature = "storage")]
    pub async fn create_credential_vault() -> Result<Arc<dyn VaultForSigning>> {
        Ok(Arc::new(SoftwareVaultForSigning::new(Arc::new(
            SecretsSqlxDatabase::create().await?,
        ))))
    }

    /// Create [`SoftwareVaultForVerifyingSignatures`]
    pub fn create_verifying_vault() -> Arc<dyn VaultForVerifyingSignatures> {
        Arc::new(SoftwareVaultForVerifyingSignatures {})
    }
}

impl Vault {
    /// Create Software Vaults and persist them to a sql database
    #[cfg(feature = "storage")]
    pub fn create_with_database(database: SqlxDatabase) -> Vault {
        Self::create_with_secrets_repository(Arc::new(SecretsSqlxDatabase::new(database)))
    }

    /// Create Software Vaults with a given secrets repository
    pub fn create_with_secrets_repository(repository: Arc<dyn SecretsRepository>) -> Vault {
        Self::new(
            Arc::new(SoftwareVaultForSigning::new(repository.clone())),
            Arc::new(SoftwareVaultForSecureChannels::new(repository.clone())),
            Arc::new(SoftwareVaultForSigning::new(repository.clone())),
            Arc::new(SoftwareVaultForVerifyingSignatures {}),
        )
    }
}
