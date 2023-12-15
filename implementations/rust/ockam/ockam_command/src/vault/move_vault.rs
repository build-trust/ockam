use clap::Args;
use colorful::Colorful;
use std::path::PathBuf;

use ockam::Context;

use crate::util::node_rpc;
use crate::{docs, fmt_err, fmt_info, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/move/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/move/after_long_help.txt");

/// Move a vault to a different path
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct MoveCommand {
    #[arg()]
    name: String,

    #[arg(long)]
    path: PathBuf,
}

impl MoveCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, MoveCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(_ctx: &Context, opts: CommandGlobalOpts, cmd: MoveCommand) -> miette::Result<()> {
    let vault_name = cmd.name;
    let vault_path = cmd.path;
    match opts
        .state
        .move_vault(&vault_name, &vault_path.clone())
        .await
    {
        Ok(()) => opts
            .terminal
            .write_line(&fmt_info!("Moved the vault {vault_name} to {vault_path:?}"))?,
        Err(e) => {
            opts.terminal.write_line(&fmt_err!(
                "Could not move the vault {vault_name} to {vault_path:?}: {e:?}"
            ))?;
            return Err(e.into());
        }
    };
    Ok(())
}
