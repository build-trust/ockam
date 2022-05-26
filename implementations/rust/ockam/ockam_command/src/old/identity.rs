use anyhow::{Context, Result};
use ockam::identity::*;

const VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IdentityFile {
    version: u32,
    identity: ExportedIdentity,
}

#[tracing::instrument(level = "debug", err)]
pub fn load_identity(ockam_dir: &std::path::Path) -> anyhow::Result<ExportedIdentity> {
    let identity_json = ockam_dir.join("identity.json");
    let ident_bytes = std::fs::read(&identity_json)
        .with_context(|| format!("Failed to open identity.json from {identity_json:?}"))?;
    let stored_ident = serde_json::from_slice::<IdentityFile>(&ident_bytes)
        .with_context(|| format!("failed to parse identity from file {identity_json:?}"))?;
    anyhow::ensure!(
        stored_ident.version == VERSION,
        "Identifier in {identity_json:?} has wrong format version",
    );
    tracing::info!(
        "Loaded identity {:?} from {:?}",
        stored_ident.identity.id,
        identity_json
    );
    Ok(stored_ident.identity)
}

#[tracing::instrument(level = "debug", skip_all, err, fields(id = ?i.id.key_id()))]
pub async fn save_identity(
    ockam_dir: &std::path::Path,
    i: &ExportedIdentity,
) -> anyhow::Result<()> {
    let ident_bytes = serde_json::to_string(&IdentityFile {
        version: VERSION,
        identity: i.clone(),
    })
    .expect("exported identity should be serializable");
    crate::old::storage::write(&ockam_dir.join("identity.json"), ident_bytes.as_bytes())?;
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
        if !crate::old::identity::is_valid_ident(line) {
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
