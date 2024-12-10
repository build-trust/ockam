use crate::cli_state::{NamedVault, VaultType};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

/// This trait allows vaults to be defined with a name and a path
/// in order to make it possible to store identity keys in different databases on disk (or in a KMS)
#[async_trait]
pub trait VaultsRepository: Send + Sync + 'static {
    /// Store a new vault path with an associated name
    async fn store_vault(&self, name: &str, vault_type: VaultType) -> Result<NamedVault>;

    /// Update a vault path
    async fn update_vault(&self, name: &str, vault_type: VaultType) -> Result<()>;

    /// Delete a vault given its name
    async fn delete_named_vault(&self, name: &str) -> Result<()>;

    /// Return the database vault if it has been created
    async fn get_database_vault(&self) -> Result<Option<NamedVault>>;

    /// Return a vault by name
    async fn get_named_vault(&self, name: &str) -> Result<Option<NamedVault>>;

    /// Return all vaults
    async fn get_named_vaults(&self) -> Result<Vec<NamedVault>>;
}

#[async_trait]
impl<T: VaultsRepository> VaultsRepository for AutoRetry<T> {
    async fn store_vault(&self, name: &str, vault_type: VaultType) -> Result<NamedVault> {
        retry!(self.wrapped.store_vault(name, vault_type.clone()))
    }

    async fn update_vault(&self, name: &str, vault_type: VaultType) -> Result<()> {
        retry!(self.wrapped.update_vault(name, vault_type.clone()))
    }

    async fn delete_named_vault(&self, name: &str) -> Result<()> {
        retry!(self.wrapped.delete_named_vault(name))
    }

    async fn get_database_vault(&self) -> Result<Option<NamedVault>> {
        retry!(self.wrapped.get_database_vault())
    }

    async fn get_named_vault(&self, name: &str) -> Result<Option<NamedVault>> {
        retry!(self.wrapped.get_named_vault(name))
    }

    async fn get_named_vaults(&self) -> Result<Vec<NamedVault>> {
        retry!(self.wrapped.get_named_vaults())
    }
}
