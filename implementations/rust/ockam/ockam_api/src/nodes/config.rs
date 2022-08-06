use crate::config::ConfigValues;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NodeManConfig {
    /// Lmdb file location
    pub authenticated_storage_path: Option<PathBuf>,
    /// Vault info
    pub vault_path: Option<PathBuf>,
    /// Exported identity value
    pub identity: Option<Vec<u8>>,
    /// Identity was overridden
    pub identity_was_overridden: bool,
}

impl ConfigValues for NodeManConfig {
    fn default_values(_config_dir: &Path) -> Self {
        Self::default()
    }
}
