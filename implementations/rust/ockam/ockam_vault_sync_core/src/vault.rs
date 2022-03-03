#[cfg(feature = "software_vault")]
use crate::VaultMutex;

// TODO(thom): this shouldn't use an anonymous struct, it's just a module.

/// Vault allows to start Vault Worker.
pub struct Vault {}

impl Vault {
    /// Start an in-memory Vault with SoftwareVault implementation.
    #[cfg(feature = "software_vault")]
    pub fn create() -> VaultMutex<ockam_vault::SoftwareVault> {
        VaultMutex::create(ockam_vault::SoftwareVault::new())
    }

    /// Initialize a vault from binary data
    #[cfg(feature = "software_vault_storage")]
    pub fn deserialize(bytes: &[u8]) -> ockam_core::Result<VaultMutex<ockam_vault::SoftwareVault>> {
        ockam_vault::SoftwareVault::deserialize(bytes).map(VaultMutex::create)
    }
}
