use crate::software_vault_impl::SoftwareVaultImpl;
use ockam_vault_core::open_close_vault::OpenCloseVault;
use ockam_vault_core::vault::Vault;

/// A pure rust implementation of a vault.
/// Is not thread-safe i.e. if multiple threads
/// add values to the vault there may be collisions
/// This is mostly for testing purposes anyway
/// and shouldn't be used for production
///
/// ```
/// use ockam_vault::software_vault::SoftwareVault;
/// let vault = SoftwareVault::default();
/// ```
#[derive(Debug)]
pub struct SoftwareVault {
    vault: SoftwareVaultImpl,
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self {
            vault: SoftwareVaultImpl::new(),
        }
    }
}

impl OpenCloseVault for SoftwareVault {
    type InnerVault = SoftwareVaultImpl;

    fn get_data_mut(&mut self) -> &mut Self::InnerVault {
        &mut self.vault
    }

    fn get_data(&self) -> &Self::InnerVault {
        &self.vault
    }

    fn open(&mut self) -> Result<Vault<'_, Self>, ockam_core::Error> {
        Ok(Vault::new(self))
    }

    fn close(&mut self) {}
}
