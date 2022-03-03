use crate::VaultMutex;

/// Vault allows to start Vault Worker.
pub struct Vault {}

impl Vault {
    /// Start a Vault with SoftwareVault implementation.
    #[cfg(feature = "software_vault")]
    pub fn create() -> VaultMutex<ockam_vault::SoftwareVault> {
        VaultMutex::create(ockam_vault::SoftwareVault::new())
    }
    #[cfg(feature = "software_vault_storage")]
    pub async fn from_path(
        p: impl AsRef<std::path::Path>,
    ) -> Result<VaultMutex<ockam_vault::SoftwareVault>, ockam_core::Error> {
        Ok(VaultMutex::create(
            ockam_vault::SoftwareVault::from_path(p).await?,
        ))
    }
}
