use ockam_core::Result;
use ockam_entity::Profile;
use ockam_node::Context;
use ockam_vault_sync_core::Vault;

async fn test(ctx: Context) -> Result<()> {
    let vault = Vault::create(&ctx).await.expect("failed to create vault");

    let _bob = Profile::create(&ctx, &vault).await?;

    Ok(())
}

fn main() {
    let (ctx, mut exec) = ockam_node::start_node();
    exec.execute(async move { test(ctx).await })
        .unwrap()
        .unwrap();
}
