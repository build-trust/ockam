use crate::software_vault::*;
use ockam_core::compat::collections::BTreeMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version")]
#[non_exhaustive]
enum SerializedVault {
    V1 {
        entries: Vec<(usize, VaultEntry)>,
        next_id: usize,
    },
}

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

impl TryFrom<SerializedVault> for VaultData {
    type Error = crate::error::VaultError;
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
                    return Err(crate::error::VaultError::StorageError);
                }
                let actual_next_id = next_id + 1;
                if map.contains_key(&actual_next_id) {
                    tracing::error!(
                        "Vault data reports {} is the next unused ID, but it's already used",
                        next_id
                    );
                    return Err(crate::error::VaultError::StorageError);
                };
                let max_id = entries.iter().map(|e| e.0).max();
                if max_id.map_or(false, |max| max >= actual_next_id) {
                    tracing::error!("Vault data reports {} is the next unused ID, but we already use IDs as high as {:?}", actual_next_id, max_id);
                    return Err(crate::error::VaultError::StorageError);
                };
                Ok(Self {
                    entries: map,
                    next_id,
                })
            }
        }
    }
}

pub(crate) fn serialize(d: &VaultData) -> Vec<u8> {
    let d = SerializedVault::from(d);
    serde_json::to_vec(&d).expect("VaultData is always serializable")
}

#[tracing::instrument(skip_all, err)]
pub(crate) fn deserialize(d: &[u8]) -> Result<VaultData, ockam_core::Error> {
    let data: SerializedVault = serde_json::from_slice(d).map_err(|e| {
        tracing::error!("Failed to deserialize saved vault JSON: {}", e);
        crate::error::VaultError::StorageError
    })?;
    Ok(VaultData::try_from(data)?)
}
