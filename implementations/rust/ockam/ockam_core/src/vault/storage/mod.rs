use crate::vault::{KeyId, SecretKey};
use crate::Result;
use crate::{async_trait, compat::boxed::Box};

/// Defines the `KeyId` interface for Ockam vaults.
#[async_trait]
pub trait Storage: Sync + Send + 'static {
    /// Store secret
    async fn store(&self, key_id: &KeyId, key: &SecretKey) -> Result<()>;
    /// Load secret
    async fn load(&self, key_id: &KeyId) -> Result<SecretKey>;
    /// Delete secret
    async fn delete(&self, key_id: &KeyId) -> Result<SecretKey>;
}
