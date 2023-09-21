use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;

use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a vault
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the vault
    pub name: String,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    let DeleteCommand { name, yes } = cmd;
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(yes, "Are you sure you want to delete this vault?")?
    {
        opts.state.vaults.get(&name)?;
        opts.state.vaults.delete(&name)?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("Vault with name '{name}' has been deleted"))
            .machine(&name)
            .json(serde_json::json!({ "vault": { "name": &name } }))
            .write_line()?;
    }
    Ok(())
}
