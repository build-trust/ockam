use ockam_entity::Entity;
use ockam_node::Context;

async fn test(ctx: Context) -> ockam_core::Result<()> {
    let mut bob = Entity::create(&ctx)?;

    let _home = bob.create_profile()?;
    Ok(())
}

fn main() {
    let (ctx, mut exec) = ockam_node::start_node();
    exec.execute(async move { test(ctx).await }).unwrap();
}
