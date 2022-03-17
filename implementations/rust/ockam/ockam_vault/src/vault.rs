#[cfg(feature = "storage")]
use crate::storage::*;
use crate::VaultError;
use core::sync::atomic::AtomicUsize;
use ockam_core::compat::{collections::BTreeMap, string::String, sync::Arc};
use ockam_core::vault::{SecretAttributes, SecretKey};
use ockam_core::Result;
use ockam_node::compat::asynchronous::RwLock;

/// Vault implementation that stores secrets in memory and uses software crypto.
///
/// # Examples
/// ```
/// use ockam_vault::Vault;
/// use ockam_core::Result;
/// use ockam_core::vault::{SecretAttributes, SecretType, SecretPersistence, CURVE25519_SECRET_LENGTH, SecretVault, Signer, Verifier};
///
/// async fn example() -> Result<()> {
///     let mut vault = Vault::default();
///
///     let mut attributes = SecretAttributes::new(
///         SecretType::X25519,
///         SecretPersistence::Ephemeral,
///         CURVE25519_SECRET_LENGTH,
///     );
///
///     let secret = vault.secret_generate(attributes).await?;
///     let public = vault.secret_public_key_get(&secret).await?;
///
///     let data = "Very important stuff".as_bytes();
///
///     let signature = vault.sign(&secret, data).await?;
///     assert!(vault.verify(&signature, &public, data).await?);
///
///     Ok(())
/// }
/// ```
#[derive(Default, Clone)]
pub struct Vault {
    pub(crate) data: VaultData,
}

#[derive(Default, Clone)]
pub(crate) struct VaultData {
    // TODO: make these private, and save automatically on modification.
    pub(crate) entries: Arc<RwLock<BTreeMap<usize, VaultEntry>>>,
    pub(crate) next_id: Arc<AtomicUsize>,
}

impl Vault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    /// Serialize a vault to bytes which may later be restored using
    /// `SoftwareVault::deserialize`.
    #[cfg(feature = "storage")]
    pub async fn serialize(&self) -> Vec<u8> {
        serialize(&self.data).await
    }

    /// Load a vault from the serialized format produced by `SoftwareVault::serialize`.
    #[cfg(feature = "storage")]
    #[tracing::instrument(err, skip_all)]
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let data = deserialize(data).map_err(|e| {
            tracing::error!("Data loaded from vault failed to parse: {:?}", e);
            VaultError::StorageError
        })?;
        Ok(Self { data })
    }

    /// Same as ```Vault::new()```
    pub fn create() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "storage", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct VaultEntry {
    key_id: Option<String>,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

impl VaultEntry {
    pub fn key_id(&self) -> &Option<String> {
        &self.key_id
    }
    pub fn key_attributes(&self) -> SecretAttributes {
        self.key_attributes
    }
    pub fn key(&self) -> &SecretKey {
        &self.key
    }
}

impl VaultEntry {
    pub fn new(key_id: Option<String>, key_attributes: SecretAttributes, key: SecretKey) -> Self {
        VaultEntry {
            key_id,
            key_attributes,
            key,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn new_vault() {
        let vault = Vault::new();
        assert_eq!(vault.data.next_id.load(Ordering::Relaxed), 0);
        assert_eq!(vault.data.entries.read().await.len(), 0);
    }
}
