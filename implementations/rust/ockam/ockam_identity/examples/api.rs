use ockam_core::Result;
use ockam_identity::Identity;
use ockam_node::Context;
use ockam_vault::Vault;

async fn test(ctx: Context) -> Result<()> {
    let vault = Vault::create();

    let _bob = Identity::create(&ctx, &vault).await?;

    Ok(())
}

fn main() {
    let (ctx, mut exec) = ockam_node::start_node();
    exec.execute(async move { test(ctx).await })
        .unwrap()
        .unwrap();
}
