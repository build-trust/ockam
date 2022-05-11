use ockam_core::compat::{collections::BTreeMap, sync::Arc};
use ockam_core::vault::storage::Storage;
use ockam_core::vault::{KeyId, SecretAttributes, SecretKey};
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
    pub(crate) storage: Option<Arc<dyn Storage>>,
}

#[derive(Default, Clone)]
pub(crate) struct VaultData {
    // TODO: make these private, and save automatically on modification.
    pub(crate) entries: Arc<RwLock<BTreeMap<KeyId, VaultEntry>>>,
}

impl Vault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        Self {
            data: Default::default(),
            storage: None,
        }
    }

    /// Same as ```Vault::new()```
    pub fn create() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) struct VaultEntry {
    key_attributes: SecretAttributes,
    key: SecretKey,
}

impl VaultEntry {
    pub fn key_attributes(&self) -> SecretAttributes {
        self.key_attributes
    }
    pub fn key(&self) -> &SecretKey {
        &self.key
    }
}

impl VaultEntry {
    pub fn new(key_attributes: SecretAttributes, key: SecretKey) -> Self {
        VaultEntry {
            key_attributes,
            key,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;

    #[tokio::test]
    async fn new_vault() {
        let vault = Vault::new();
        assert_eq!(vault.data.entries.read().await.len(), 0);
    }
}
