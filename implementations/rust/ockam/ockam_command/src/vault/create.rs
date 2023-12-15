use clap::Args;
use colorful::Colorful;
use std::path::PathBuf;

use ockam::Context;

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
    #[arg()]
    name: Option<String>,

    #[arg(long)]
    path: Option<PathBuf>,

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
    if opts.state.get_named_vaults().await?.is_empty() {
        opts.terminal.write_line(&fmt_info!(
            "This is the first vault to be created in this environment. It will be set as the default vault"
        ))?;
    }
    let vault = if cmd.aws_kms {
        opts.state.create_kms_vault(&cmd.name, &cmd.path).await?
    } else {
        opts.state.create_named_vault(&cmd.name, &cmd.path).await?
    };

    opts.terminal
        .stdout()
        .plain(fmt_ok!("Vault created with name '{}'!", vault.name()))
        .machine(vault.name())
        .json(serde_json::json!({ "name": &cmd.name }))
        .write_line()?;
    Ok(())
}
