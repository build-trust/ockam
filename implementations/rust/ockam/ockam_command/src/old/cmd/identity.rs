use clap::Args;

use ockam::Context;

use crate::old::identity::create_identity;

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
    create_identity(&ctx, args.overwrite).await?;
    ctx.stop().await?;
    Ok(())
}
