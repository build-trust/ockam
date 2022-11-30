use crate::util::node_rpc;
use crate::CommandGlobalOpts;
use crate::Result;
use clap::Args;
use ockam::Context;
use ockam_api::cli_state;
use rand::prelude::random;

/// Create vaults
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(_ctx: Context, (options, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    let path = cli_state::VaultConfig::fs_path(&cmd.name, cmd.path)?;
    let config = cli_state::VaultConfig::fs(path, cmd.aws_kms)?;
    options
        .state
        .vaults
        .create(&cmd.name, config.clone())
        .await?;
    println!("Vault created: {}", &cmd.name);
    Ok(())
}
