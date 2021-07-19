use ockam_entity::Entity;
use ockam_node::Context;
use ockam_vault_sync_core::Vault;

async fn test(ctx: Context) -> ockam_core::Result<()> {
    let vault = Vault::create(&ctx).expect("failed to create vault");

    let mut bob = Entity::create(&ctx, &vault)?;

    let _home = bob.create_profile(&vault)?;
    Ok(())
}

fn main() {
    let (ctx, mut exec) = ockam_node::start_node();
    exec.execute(async move { test(ctx).await }).unwrap();
}
