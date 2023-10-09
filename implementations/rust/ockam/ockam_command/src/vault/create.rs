use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state;
use ockam_api::cli_state::random_name;
use ockam_api::cli_state::traits::StateDirTrait;

use crate::util::node_rpc;
use crate::{docs, fmt_info, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a vault
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[arg(hide_default_value = true, default_value_t = random_name())]
    name: String,

    #[arg(long, default_value = "false")]
    aws_kms: bool,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let CreateCommand { name, aws_kms, .. } = cmd;
    let config = cli_state::VaultConfig::new(aws_kms)?;
    if opts.state.vaults.is_empty()? {
        opts.terminal.write_line(&fmt_info!(
            "This is the first vault to be created in this environment. It will be set as the default vault"
        ))?;
    }
    opts.state
        .vaults
        .create_async(&name, config.clone())
        .await?;

    opts.terminal
        .stdout()
        .plain(fmt_ok!("Vault created with name '{name}'!"))
        .machine(&name)
        .json(serde_json::json!({ "vault": { "name": &name } }))
        .write_line()?;
    Ok(())
}
