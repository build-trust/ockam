use ockam::Context;

use crate::{args::IdentityOpts, identity::create_identity, storage};

pub async fn run(args: IdentityOpts, mut ctx: Context) -> anyhow::Result<()> {
    let ockam_dir = storage::init_ockam_dir()?;
    create_identity(&args, &ctx, ockam_dir).await?;
    ctx.stop().await?;
    Ok(())
}
