use crate::storage::*;
use anyhow::Context;
use std::path::{Path, PathBuf};
use tokio::io::*;

// #[tracing::instrument(level = "debug", err)]
// pub async fn open(vault_json: &Path) -> anyhow::Result<OckamVault> {
//     let bytes = tokio::fs::read(vault_json)
//         .await
//         .with_context(|| format!("Failed to open vault file at {:?}", vault_json))?;
//     let vault = ockam::Vault::deserialize(&bytes).with_context(|| {
//         format!("Failed to parse data from vault located at {:?}.", vault_json);
//     })?;
//     Ok(vault)
// }
