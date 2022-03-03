use crate::{args::IdentityOpts, identity::save_identity, storage};
use anyhow::Context as Ctx;
use ockam::{identity::*, vault::*, Context};

pub async fn run(args: IdentityOpts, mut ctx: Context) -> anyhow::Result<()> {
    let ockam_dir = storage::init_ockam_dir()?;
    let id_path = ockam_dir.join("identity.json");
    let vault_path = ockam_dir.join("vault.json");
    if id_path.exists() || vault_path.exists() {
        if !args.overwrite {
            anyhow::bail!(
                "An identity or vault already exists in {:?}. Pass `--overwrite` to continue anyway",
                ockam_dir
            );
        }
        if vault_path.exists() {
            std::fs::remove_file(&vault_path).with_context(|| format!("Failed to remove {:?}", vault_path))?;
        }
        if id_path.exists() {
            std::fs::remove_file(&id_path).with_context(|| format!("Failed to remove {:?}", id_path))?;
        }
    }
    let vault = Vault::create();
    let identity = Identity::create(&ctx, &vault).await?;
    let exported = identity.export().await;
    let identifier = exported.id.clone();
    tracing::info!("Saving new identity: {:?}", identifier.key_id());
    save_identity(&ockam_dir, &exported, &vault).await?;
    println!("Initialized {:?} with identity {:?}.", ockam_dir, identifier.key_id());
    ctx.stop().await?;
    Ok(())
}
