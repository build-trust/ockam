use core::sync::atomic::AtomicUsize;
use ockam_core::compat::{collections::BTreeMap, string::String, sync::Arc};
use ockam_core::vault::{SecretAttributes, SecretKey};
use ockam_node::compat::asynchronous::RwLock;
use tracing::info;

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
#[derive(Clone, Debug)]
pub struct Vault {
    pub(crate) entries: Arc<RwLock<BTreeMap<usize, VaultEntry>>>,
    pub(crate) next_id: Arc<AtomicUsize>,
}

impl Vault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        info!("Creating vault");
        Self {
            entries: Default::default(),
            next_id: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Same as ```Vault::new()```
    pub fn create() -> Self {
        Self::new()
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
        assert_eq!(vault.next_id.load(Ordering::Relaxed), 0);
        assert_eq!(vault.entries.read().await.len(), 0);
    }
}
