use ockam_core::compat::{collections::BTreeMap, sync::Arc};
use ockam_core::vault::storage::Storage;
use ockam_core::vault::{KeyId, VaultEntry};
use ockam_node::compat::asynchronous::RwLock;

/// Vault implementation that stores secrets in memory and uses software crypto.
///
/// # Examples
/// ```
/// use ockam_vault::Vault;
/// use ockam_core::Result;
/// use ockam_core::vault::{SecretAttributes, SecretType, SecretPersistence, CURVE25519_SECRET_LENGTH_U32, SecretVault, Signer, Verifier};
///
/// async fn example() -> Result<()> {
///     let mut vault = Vault::default();
///
///     let mut attributes = SecretAttributes::new(
///         SecretType::X25519,
///         SecretPersistence::Ephemeral,
///         CURVE25519_SECRET_LENGTH_U32,
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
    #[cfg(feature = "aws")]
    pub(crate) aws_kms: Option<crate::aws::Kms>,
}

#[derive(Default, Clone)]
pub(crate) struct VaultData {
    pub(crate) entries: Arc<RwLock<BTreeMap<KeyId, VaultEntry>>>,
}

impl Vault {
    /// Create a new SoftwareVault
    pub fn new(storage: Option<Arc<dyn Storage>>) -> Self {
        Self {
            data: Default::default(),
            storage,
            #[cfg(feature = "aws")]
            aws_kms: None,
        }
    }

    /// Same as ```Vault::new()```
    pub fn create() -> Self {
        Self::new(None)
    }

    /// Enable AWS KMS.
    #[cfg(feature = "aws")]
    pub async fn enable_aws_kms(&mut self) -> Result<(), ockam_core::Error> {
        let kms = crate::aws::Kms::default().await?;
        self.aws_kms = Some(kms);
        Ok(())
    }

    pub(crate) async fn preload_from_storage(&self, key_id: &KeyId) {
        // Do nothing if there is no Storage
        let storage = match &self.storage {
            Some(s) => s,
            None => return,
        };

        // Check if given secret is already loaded into the memory
        if self.data.entries.read().await.contains_key(key_id) {
            return;
        }

        // Try to load secret from the Storage
        let entry = match storage.load(key_id).await {
            Ok(e) => e,
            Err(_) => return,
        };

        // It's fine if we override the values, since storage is expected to return the same
        // data for the given key_id
        let _ = self
            .data
            .entries
            .write()
            .await
            .insert(key_id.clone(), entry);
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;

    #[tokio::test]
    async fn new_vault() {
        let vault = Vault::create();
        assert_eq!(vault.data.entries.read().await.len(), 0);
    }
}
