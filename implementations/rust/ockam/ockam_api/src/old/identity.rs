use std::path::{Path, PathBuf};

use crate::error::ApiError;
use ockam::identity::*;
use ockam_core::Result;
use ockam_node::Context;
use ockam_vault::Vault;

const VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IdentityFile {
    version: u32,
    identity: Vec<u8>,
}

pub async fn create_identity(
    ctx: &Context,
    ockam_dir: &PathBuf,
    vault: &Vault,
    overwrite: bool,
) -> Result<Identity<Vault>> {
    let id_path = ockam_dir.join("identity.json");
    if id_path.exists() {
        if !overwrite {
            return Err(ApiError::generic(
                     "An identity or vault already exists in {ockam_dir:?}. Pass `--overwrite` to continue anyway",
            ));
        }
        if id_path.exists() {
            std::fs::remove_file(&id_path)
                .map_err(|_| ApiError::generic("Error removing identity file {id_path:?}"))?;
        }
    }

    let identity = Identity::create(ctx, vault).await?;
    let identifier = identity.identifier()?;
    tracing::debug!("Saving new identity: {:?}", identifier.key_id());
    save_identity(ockam_dir, &identity).await?;
    tracing::info!(
        "Initialized {:?} with identity {:?}.",
        ockam_dir,
        identifier.key_id()
    );
    Ok(identity)
}

pub async fn load_identity(
    ctx: &Context,
    ockam_dir: &Path,
    vault: &Vault,
) -> Result<Identity<Vault>> {
    let identity_json = ockam_dir.join("identity.json");
    let ident_bytes = std::fs::read(&identity_json)
        .map_err(|_| ApiError::generic("Failed to open identity.json from {identity_json:?}"))?;
    let stored_ident = serde_json::from_slice::<IdentityFile>(&ident_bytes)
        .map_err(|_| ApiError::generic("failed to parse identity from file {identity_json:?}"))?;
    if stored_ident.version != VERSION {
        return Err(ApiError::generic(
            "Identifier in {identity_json:?} has wrong format version",
        ));
    }
    let identity = Identity::import(ctx, &stored_ident.identity, vault).await?;
    tracing::info!("Loaded identity {:?}", identity.identifier()?,);
    Ok(identity)
}

async fn save_identity(ockam_dir: &Path, i: &Identity<Vault>) -> Result<()> {
    let ident_bytes = serde_json::to_string(&IdentityFile {
        version: VERSION,
        identity: i.export().await?,
    })
    .expect("exported identity should be serializable");
    crate::old::storage::write(&ockam_dir.join("identity.json"), ident_bytes.as_bytes())?;
    Ok(())
}
