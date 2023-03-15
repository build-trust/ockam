use clap::Args;
use rand::prelude::random;

use ockam::Context;
use ockam_api::cli_state;

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    name: String,

    /// Path to the Vault storage file
    #[arg(short, long)]
    path: Option<String>,

    #[arg(long, default_value = "false")]
    aws_kms: bool,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> crate::Result<()> {
    let config = cli_state::VaultConfig::new(cmd.aws_kms)?;
    opts.state.vaults.create(&cmd.name, config.clone()).await?;
    println!("Vault created: {}", &cmd.name);
    Ok(())
}
