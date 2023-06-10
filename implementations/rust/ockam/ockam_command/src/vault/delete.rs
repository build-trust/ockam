use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliStateError;

use crate::terminal::ConfirmResult;
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    _ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> crate::Result<()> {
    let state = opts.state.vaults;
    let name = cmd.name;
    if cmd.yes {
        match state.get(&name) {
            // If it exists, proceed
            Ok(_) => {
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
                CliStateError::NotFound => Err(miette!("Vault '{}' not found", &name).into()),
                _ => Err(err.into()),
            },
        }
    } else {
        // If yes is not provided make sure using TTY
        match state.get(&name) {
            // If it exists, proceed
            Ok(_) => {
                match opts
                    .terminal
                    .confirm("This will delete the selected Vault. Are you sure?")?
                {
                    ConfirmResult::Yes => {}
                    ConfirmResult::No => {
                        return Ok(());
                    }
                    ConfirmResult::NonTTY => {
                        return Err(miette!("Use --yes to confirm").into());
                    }
                }
                state.delete(&name)?;
                Ok(())
            }
            // Else, return the appropriate error
            Err(err) => match err {
                CliStateError::NotFound => Err(miette!("Vault '{}' not found", &name).into()),
                _ => Err(err.into()),
            },
        }
    }
}
