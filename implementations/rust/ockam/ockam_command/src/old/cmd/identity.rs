use crate::old::{identity::save_identity, storage, OckamVault};
use anyhow::Context as Ctx;
use clap::Args;
use ockam::{identity::*, Context};
use ockam_vault::storage::FileStorage;
use std::sync::Arc;

#[derive(Clone, Debug, Args)]
pub struct IdentityOpts {
    /// If an ockam identity already exists, overwrite it.
    ///
    /// This is a destructive operation and cannot be undone.
    ///
    /// Note: This only applies to the `<ockam_dir>/identity.json` files,
    /// and not to `<ockam_dir>/trusted`, which is left as-is must be managed manually.
    /// For example, with the `ockam add-trusted-identity` subcommand)
    #[clap(long)]
    pub overwrite: bool,
}

pub async fn run(args: IdentityOpts, mut ctx: Context) -> anyhow::Result<()> {
    let ockam_dir = storage::init_ockam_dir()?;
    let id_path = ockam_dir.join("identity.json");
    if id_path.exists() {
        if !args.overwrite {
            anyhow::bail!(
                "An identity or vault already exists in {:?}. Pass `--overwrite` to continue anyway",
                ockam_dir
            );
        }
        if id_path.exists() {
            std::fs::remove_file(&id_path)
                .with_context(|| format!("Failed to remove {:?}", id_path))?;
        }
    }
    let vault_storage = FileStorage::create(
        &ockam_dir.join("vault.json"),
        &ockam_dir.join("vault.json.temp"),
    )
    .await?;
    let vault = OckamVault::new(Some(Arc::new(vault_storage)));
    let identity = Identity::create(&ctx, &vault).await?;
    let exported = identity.export().await;
    let identifier = exported.id.clone();
    tracing::info!("Saving new identity: {:?}", identifier.key_id());
    save_identity(&ockam_dir, &exported).await?;
    println!(
        "Initialized {:?} with identity {:?}.",
        ockam_dir,
        identifier.key_id()
    );
    ctx.stop().await?;
    Ok(())
}
