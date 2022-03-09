#[cfg(feature = "software_vault")]
use crate::VaultMutex;

/// Vault allows to start Vault Worker.
pub struct Vault {}

impl Vault {
    /// Start a Vault with SoftwareVault implementation.
    #[cfg(feature = "software_vault")]
    pub fn create() -> VaultMutex<ockam_vault::SoftwareVault> {
        VaultMutex::create(ockam_vault::SoftwareVault::new())
    }
}
