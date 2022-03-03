use crate::storage::*;
use anyhow::Context;
use ockam::ExportedIdentity;
pub type OckamVault = ockam::VaultMutex<ockam_vault::SoftwareVault>;

const VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IdentityFile {
    version: u32,
    identity: ExportedIdentity,
}

#[tracing::instrument(level = "debug", err)]
pub fn load_identity(ockam_dir: &std::path::Path) -> anyhow::Result<(ExportedIdentity, OckamVault)> {
    let ident_path = ockam_dir.join("identity.json");
    let vault_path = ockam_dir.join("vault.json");

    let ident_bytes =
        std::fs::read(&ident_path).with_context(|| format!("Failed to open identity.json from {ident_path:?}"))?;
    let vault_bytes =
        std::fs::read(&vault_path).with_context(|| format!("Failed to open vault.json from {vault_path:?}"))?;

    let vault = ockam::Vault::deserialize(&vault_bytes[..])
        .with_context(|| format!("Failed to load the ockam vault at {vault_path:?}"))?;

    let stored_ident = serde_json::from_slice::<IdentityFile>(&ident_bytes)
        .with_context(|| format!("failed to parse identity from file {ident_path:?}"))?;

    anyhow::ensure!(
        stored_ident.version == VERSION,
        "Identifier in {ident_path:?} has wrong format version",
    );
    tracing::info!("Loaded identity {:?} from {:?}", stored_ident.identity.id, ident_path);
    Ok((stored_ident.identity, vault))
}

#[tracing::instrument(level = "debug", skip_all, err, fields(id = ?i.id.key_id()))]
pub async fn save_identity(
    ockam_dir: &std::path::Path,
    i: &ExportedIdentity,
    vault: &OckamVault,
) -> anyhow::Result<()> {
    let vault_bytes = vault.lock().await.serialize();
    let ident_bytes = serde_json::to_string(&IdentityFile {
        version: VERSION,
        identity: i.clone(),
    })
    .expect("exported identity should be serializable");
    crate::storage::write(&ockam_dir.join("identity.json"), ident_bytes.as_bytes())?;
    crate::storage::write(&ockam_dir.join("vault.json"), &vault_bytes)?;
    Ok(())
}
