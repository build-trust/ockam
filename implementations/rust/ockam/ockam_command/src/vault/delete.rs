use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliStateError;

use crate::terminal::ConfirmResult;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, fmt_warn, CommandGlobalOpts};

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
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    let DeleteCommand { name } = cmd;
    let state = opts.state.vaults;
    match state.get(&name) {
        // If it exists, proceed
        Ok(_) => {
            if let ConfirmResult::No = opts.terminal.confirm(&fmt_warn!(
                "This will delete the vault with name '{name}'. Do you want to continue?"
            ))? {
                // If the user has not confirmed, exit
                return Ok(());
            }

            state.delete(&name)?;

            opts.terminal
                .stdout()
                .plain(fmt_ok!("Vault with name '{name}' has been deleted"))
                .machine(&name)
                .json(serde_json::json!({ "vault": { "name": &name } }))
                .write_line()?;

            Ok(())
        }
        // Else, return the appropriate error
        Err(err) => match err {
            CliStateError::NotFound => Err(miette!("Vault '{}' not found", name)),
            _ => Err(err.into()),
        },
    }
}
