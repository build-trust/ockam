use crate::vault::{KeyId, VaultEntry};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines Storage interface for Ockam vaults.
#[async_trait]
pub trait Storage: Sync + Send + 'static {
    /// Store secret
    async fn store(&self, key_id: &KeyId, key: &VaultEntry) -> Result<()>;
    /// Load secret
    async fn load(&self, key_id: &KeyId) -> Result<VaultEntry>;
    /// Delete secret
    async fn delete(&self, key_id: &KeyId) -> Result<VaultEntry>;
}
