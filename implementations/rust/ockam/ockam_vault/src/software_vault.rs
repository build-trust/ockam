use crate::VaultError;
use ockam_core::compat::{collections::BTreeMap, string::String};
use ockam_core::Result;
use ockam_vault_core::{Secret, SecretAttributes, SecretKey};
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
#[derive(Debug)]
pub struct SoftwareVault {
    pub(crate) entries: BTreeMap<usize, VaultEntry>,
    pub(crate) next_id: usize,
}

impl SoftwareVault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        info!("Creating vault");
        Self {
            entries: Default::default(),
            next_id: 0,
        }
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareVault {
    pub(crate) fn get_entry(&self, context: &Secret) -> Result<&VaultEntry> {
        self.entries
            .get(&context.index())
            .ok_or_else(|| VaultError::EntryNotFound.into())
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
        assert_eq!(vault.next_id, 0);
        assert_eq!(vault.entries.len(), 0);
    }
}
