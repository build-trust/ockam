use crate::OckamVault;
use anyhow::{Context, Result};
use ockam::{identity::*, vault::*};

const VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IdentityFile {
    version: u32,
    identity: ExportedIdentity,
}

// #[tracing::instrument(level = "debug", err)]
pub fn load_identity(identity_json: &std::path::Path) -> anyhow::Result<ExportedIdentity> {
    let ident_bytes = std::fs::read(&identity_json)
        .with_context(|| format!("Failed to open identity.json from {identity_json:?}"))?;
    let stored_ident = serde_json::from_slice::<IdentityFile>(&ident_bytes)
        .with_context(|| format!("failed to parse identity from file {identity_json:?}"))?;
    anyhow::ensure!(
        stored_ident.version == VERSION,
        "Identifier in {identity_json:?} has wrong format version",
    );
    Ok(stored_ident.identity)
}

#[tracing::instrument(level = "debug", err)]
pub fn load_identity_and_vault(
    ockam_dir: &std::path::Path,
) -> anyhow::Result<(ExportedIdentity, OckamVault)> {
    let vault_path = ockam_dir.join("vault.json");
    let vault_bytes = std::fs::read(&vault_path)
        .with_context(|| format!("Failed to open vault.json from {vault_path:?}"))?;
    let vault = Vault::deserialize(&vault_bytes[..])
        .with_context(|| format!("Failed to load the ockam vault at {vault_path:?}"))?;
    let ident_path = ockam_dir.join("identity.json");
    let identity = load_identity(&ident_path)?;
    tracing::info!("Loaded identity {:?} from {:?}", identity.id, ident_path);
    Ok((identity, vault))
}

#[tracing::instrument(level = "debug", skip_all, err, fields(id = ?i.id.key_id()))]
pub async fn save_identity(
    ockam_dir: &std::path::Path,
    i: &ExportedIdentity,
    vault: &OckamVault,
) -> anyhow::Result<()> {
    let vault_bytes = vault.serialize().await;
    let ident_bytes = serde_json::to_string(&IdentityFile {
        version: VERSION,
        identity: i.clone(),
    })
    .expect("exported identity should be serializable");
    crate::storage::write(&ockam_dir.join("identity.json"), ident_bytes.as_bytes())?;
    crate::storage::write(&ockam_dir.join("vault.json"), &vault_bytes)?;
    Ok(())
}

pub fn parse_identities(idents: &str) -> Result<Vec<IdentityIdentifier>> {
    idents
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if !is_valid_ident(s) {
                Err(anyhow::anyhow!(
                    "Failed to parse identifier (should be 64 hexadecimal ASCII characters): {s:?}"
                ))
            } else {
                Ok(IdentityIdentifier::from_key_id(s.into()))
            }
        })
        .collect()
}

pub fn is_valid_ident(s: &str) -> bool {
    matches!(hex::decode(s), Ok(v) if v.len() == 32)
}

pub fn read_trusted_idents_from_file(
    path: &std::path::Path,
) -> anyhow::Result<Vec<IdentityIdentifier>> {
    // No TOCTOU here, this is just for a better error message.
    if !path.exists() {
        anyhow::bail!("No trusted identifiers list exists at {path:?}.");
    }
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to open trusted identifier file `{path:?}`"))?;
    let lines = data
        .lines()
        .enumerate()
        .map(|l| (l.0, l.1.trim()))
        .filter(|(_, l)| !l.is_empty())
        .map(|(n, id)| (n, id.strip_prefix('P').unwrap_or(id)));
    let mut idents = vec![];
    for (num, line) in lines {
        if !crate::identity::is_valid_ident(line) {
            anyhow::bail!(
                "Failed to parse '{path:?}'. Line {num} is not a valid identifier. \
                Expected 64 ascii hex chars, but got: {line:?}",
            );
        }
        let ident = IdentityIdentifier::from_key_id(line.into());
        idents.push(ident);
    }
    Ok(idents)
}
