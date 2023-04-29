use clap::Args;
use colorful::Colorful;
use rand::prelude::random;

use ockam::Context;
use ockam_api::cli_state;
use ockam_api::cli_state::traits::StateTrait;

use crate::util::node_rpc;
use crate::{fmt_info, fmt_ok, CommandGlobalOpts};

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
    let CreateCommand { name, aws_kms, .. } = cmd;
    let config = cli_state::VaultConfig::new(aws_kms)?;
    if !opts.state.vaults.has_default()? {
        opts.terminal.write_line(&fmt_info!(
            "This is the first vault to be created in this environment. It will be set as the default vault"
        ))?;
    }
    opts.state.vaults.create(&name, config.clone()).await?;

    opts.terminal
        .stdout()
        .plain(fmt_ok!("Vault created with name '{name}'!"))
        .machine(&name)
        .json(serde_json::json!({ "vault": { "name": &name } }))
        .write_line()?;
    Ok(())
}
