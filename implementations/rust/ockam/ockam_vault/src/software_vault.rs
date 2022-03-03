use crate::VaultError;
use ockam_core::compat::{collections::BTreeMap, string::String};
use ockam_core::vault::{Secret, SecretAttributes, SecretKey};
use ockam_core::Result;
use tracing::info;

/// Vault implementation that stores secrets in memory and uses software crypto.
///
/// # Examples
/// ```
/// use ockam_vault::SoftwareVault;
/// use ockam_core::Result;
/// use ockam_core::vault::{SecretAttributes, SecretType, SecretPersistence, CURVE25519_SECRET_LENGTH, SecretVault, Signer, Verifier};
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
    pub(crate) data: VaultData,
    // FIXME: we should have some sort of generic VaultStorage trait instead.
    #[cfg(feature = "storage")]
    file: Option<tokio::fs::File>,
}

#[derive(Default, Debug)]
pub(crate) struct VaultData {
    // TODO: make these private, and save automatically on modification.
    pub(crate) entries: BTreeMap<usize, VaultEntry>,
    pub(crate) next_id: usize,
}

#[cfg(feature = "storage")]
impl From<&VaultData> for SerializedVault {
    fn from(d: &VaultData) -> SerializedVault {
        let entries = d
            .entries
            .iter()
            .map(|(sid, data)| (*sid, data.clone()))
            .collect();
        SerializedVault::V1 {
            entries,
            next_id: d.next_id,
        }
    }
}

#[cfg(feature = "storage")]
impl TryFrom<SerializedVault> for VaultData {
    type Error = std::io::Error;
    fn try_from(v: SerializedVault) -> Result<Self, Self::Error> {
        match v {
            SerializedVault::V1 { entries, next_id } => {
                let map: BTreeMap<usize, VaultEntry> = entries.iter().cloned().collect();
                if map.len() != entries.len() {
                    tracing::error!(
                        "Duplicate secret ID in vault data ({} entries, {} unique)",
                        entries.len(),
                        map.len()
                    );
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "duplicate secret id in vault data",
                    ));
                }
                if map.contains_key(&next_id) {
                    tracing::error!(
                        "Vault data reports {} is the next unused ID, but it's already used",
                        next_id
                    );
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Vault next_id not unique",
                    ));
                };
                let max_id = entries.iter().map(|e| e.0).max();
                if max_id.map_or(false, |max| max >= next_id) {
                    tracing::error!("Vault data reports {} is the next unused ID, but we already use IDs as high as {:?}", next_id, max_id);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Vault data next_id will collide",
                    ));
                };
                Ok(Self {
                    entries: map,
                    next_id,
                })
            }
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "storage")]
enum SerializedVault {
    V1 {
        entries: Vec<(usize, VaultEntry)>,
        next_id: usize,
    },
}

impl SoftwareVault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        info!("Creating vault (no storage)");
        Self {
            data: Default::default(),
            #[cfg(feature = "storage")]
            file: None,
        }
    }

    /// Create or load a software vault from the specified path.
    ///
    /// If the path file does not exist, it will be created. Changes to the
    /// vault will be persisted to this location. Because the data is sensitive,
    /// it is persisted with permissions `0o600`.
    #[cfg(feature = "storage")]
    pub async fn from_path(f: impl AsRef<std::path::Path>) -> Result<Self> {
        let vault = Self::from_path_impl(f.as_ref())
            .await
            .map_err(|_| ockam_core::Error::from(crate::VaultError::StorageError))?;
        Ok(vault)
    }

    #[cfg(feature = "storage")]
    #[tracing::instrument(err)]
    async fn from_path_impl(path: &std::path::Path) -> Result<Self, tokio::io::Error> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
        info!(
            "Loading (or initializing) vault with storage at at {}",
            path.display()
        );
        // TODO: take out file lock
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            // rw for owner, nothing for anybody else
            .mode(0o600)
            .open(path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open vault at {path:?}: {e}");
                e
            })?;
        info!("Creating vault with storage at at {}", path.display());
        let mut text = String::new();
        let len = file.read_to_string(&mut text).await?;
        let data = if len == 0 || text.trim().is_empty() {
            tracing::debug!("Writing initial data to vault at {}", path.display());
            let items = VaultData::default();
            // TODO: dupe of code in `save()`
            file.rewind().await?;
            let serialized =
                serde_json::to_vec(&SerializedVault::from(&items)).expect("failed to serialize?");
            file.write_all(&serialized).await?;
            file.flush().await?;
            items
        } else {
            tracing::debug!("Loading data from vault at {}", path.display());
            let ser: SerializedVault = serde_json::from_str(&text).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid JSON in vault file ({}): {}", path.display(), e),
                )
            })?;
            let data: VaultData = ser.try_into()?;
            data
        };
        Ok(Self {
            data,
            file: Some(file),
        })
    }

    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn save(&mut self) -> Result<(), tokio::io::Error> {
        #[cfg(feature = "storage")]
        if let Some(file) = self.file.as_mut() {
            use tokio::io::{AsyncSeekExt, AsyncWriteExt};
            tracing::debug!("Saving data to vault...");
            file.rewind().await?;
            let serialized = serde_json::to_vec(&SerializedVault::from(&self.data))
                .expect("failed to serialize?");
            file.write_all(&serialized).await?;
            file.flush().await?;
            tracing::debug!("Saved data to vault.");
        }
        Ok(())
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareVault {
    pub(crate) fn get_entry(&self, context: &Secret) -> Result<&VaultEntry> {
        self.data
            .entries
            .get(&context.index())
            .ok_or_else(|| VaultError::EntryNotFound.into())
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
    use crate::SoftwareVault;

    #[test]
    fn new_vault() {
        let vault = SoftwareVault::new();
        assert_eq!(vault.data.next_id, 0);
        assert_eq!(vault.data.entries.len(), 0);
    }
}
