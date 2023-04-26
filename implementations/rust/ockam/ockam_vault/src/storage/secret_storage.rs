use crate::{KeyId, VaultEntry};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// This trait provides an interface for storing secrets.
///
/// A SecretStorage makes the distinction between persistent and ephemeral secrets:
///   - persistent secrets are persisted to long-term memory (a cache can be used by the implementation for faster access)
///   - ephemeral secrets are only stored in-memory and lost when the node is stopped
///
#[async_trait]
pub(crate) trait SecretStorage: Sync + Send + 'static {
    /// Store a secret
    async fn store_secret(&self, key_id: &KeyId, key: &VaultEntry) -> Result<()>;

    /// Get a secret, whether persistent or ephemeral given its key
    /// Return an error using the description if the secret is not found
    async fn get_secret(&self, secret: &KeyId, description: &str) -> Result<VaultEntry>;

    /// Get a persistent secret given its key
    /// Return an error using the description if the secret is not found
    async fn get_persistent_secret(&self, secret: &KeyId, description: &str) -> Result<VaultEntry>;

    /// Get an ephemeral secret given its key
    /// Return an error using the description if the secret is not found
    async fn get_ephemeral_secret(&self, secret: &KeyId, description: &str) -> Result<VaultEntry>;

    /// Delete a secret, either from memory or persistent storage
    async fn delete_secret(&self, key_id: &KeyId) -> Result<Option<VaultEntry>>;
}
