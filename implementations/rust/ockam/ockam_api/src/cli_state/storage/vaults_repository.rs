use std::path::Path;

use ockam_core::async_trait;
use ockam_core::Result;

use crate::NamedVault;

/// This trait allows vaults to be defined with a name and a path
/// in order to make it possible to store identity keys in different databases on disk (or in a KMS)
#[async_trait]
pub trait VaultsRepository: Send + Sync + 'static {
    /// Store a new vault path with an associated name
    async fn store_vault(&self, name: &str, path: &Path, is_kms: bool) -> Result<NamedVault>;

    /// Update a vault path
    async fn update_vault(&self, name: &str, path: &Path) -> Result<()>;

    /// Delete a vault given its name
    async fn delete_named_vault(&self, name: &str) -> Result<()>;

    /// Return a vault by name
    async fn get_named_vault(&self, name: &str) -> Result<Option<NamedVault>>;

    /// Return all vaults
    async fn get_named_vaults(&self) -> Result<Vec<NamedVault>>;
}
