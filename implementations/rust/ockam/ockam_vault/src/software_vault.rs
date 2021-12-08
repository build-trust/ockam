use crate::VaultError;
use ockam_core::compat::{collections::BTreeMap, string::String};
use ockam_core::Result;
use ockam_vault_core::{Secret, SecretAttributes, SecretKey};
use ockam_core::compat::sync::RwLock;
use ockam_core::compat::sync::Arc;
use tracing::info;

/// Vault implementation that stores secrets in memory and uses software crypto.
///
/// # Examples
/// ```
/// use ockam_vault::SoftwareVault;
/// use ockam_core::Result;
/// use ockam_vault_core::{SecretAttributes, SecretType, SecretPersistence, CURVE25519_SECRET_LENGTH, SecretVault, Signer, Verifier};
///
/// async fn example() -> Result<()> {
///     let mut vault = SoftwareVault::default();
///
///     let mut attributes = SecretAttributes::new(
///         SecretType::Curve25519,
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
pub struct SoftwareVault {
    pub(crate) inner: RwLock<VaultStorage>,
}

pub(crate) struct VaultStorage {
    pub(crate) entries: BTreeMap<usize, VaultEntry>,
    next_id: usize,
}

impl SoftwareVault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        info!("Creating vault");
        Self {
            inner: RwLock::new(VaultStorage {
                entries: BTreeMap::new(),
                next_id: 0,
            })
        }
    }

    /// Create a new `Arc<SoftwareVault>`
    pub fn new_arc() -> Arc<Self> {
        Self::new().into()
    }

    pub(crate) fn insert(&self, entry: VaultEntry) -> Secret {
        let mut storage = self.inner.write();
        let next_id = storage.next_id + 1;
        storage.next_id = next_id;
        storage.entries.insert(next_id, entry);
        Secret::new(next_id)
    }

    pub(crate) fn remove(&self, entry: Secret) -> Option<VaultEntry> {
        let mut storage = self.inner.write();
        storage.entries.remove(&entry.index())
    }

    // TODO: we only need this because we don't have mapped guards on `std`.
    pub(crate) fn with_entry<Ret, F: FnOnce(&VaultEntry) -> Ret>(&self, secret: &Secret, reader: F) -> Result<Ret, VaultError> {
        let storage = self.inner.read();
        let entry = storage.get_entry(&secret)?;
        Ok(reader(entry))
    }
}

impl VaultStorage {
    pub(crate) fn get_entry<'a>(&'a self, secret: &Secret) -> Result<&'a VaultEntry, VaultError> {
        self.entries.get(&secret.index()).ok_or(VaultError::EntryNotFound)
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq)]
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
    use crate::SoftwareVault;

    #[test]
    fn new_vault() {
        let vault = SoftwareVault::new();
        assert_eq!(vault.inner.read().next_id, 0);
        assert_eq!(vault.inner.read().entries.len(), 0);
    }
}
