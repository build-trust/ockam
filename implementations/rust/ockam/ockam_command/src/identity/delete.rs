use crate::terminal::ConfirmResult;
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliStateError;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete an identity
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name of the identity to be deleted
    name: String,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let state = opts.state;
    let name = cmd.name;
    // Check if --yes flag is provided
    if cmd.yes {
        // check if exists
        match state.identities.get(&name) {
            Ok(identity_state) => {
                state.delete_identity(identity_state)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Identity with name '{name}' has been deleted"))
                    .machine(&name)
                    .json(serde_json::json!({ "vault": { "name": &name } }))
                    .write_line()?;
                Ok(())
            }
            // Return the appropriate error
            Err(err) => match err {
                CliStateError::NotFound => Err(miette!("Identity '{}' not found", &name).into()),
                _ => Err(err.into()),
            },
        }
    } else {
        // If yes is not provided make sure using TTY
        match state.identities.get(&name) {
            // If it exists, proceed
            Ok(identity_state) => {
                match opts
                    .terminal
                    .confirm("This will delete the selected Identity. Are you sure?")?
                {
                    ConfirmResult::Yes => {}
                    ConfirmResult::No => {
                        return Ok(());
                    }
                    ConfirmResult::NonTTY => {
                        return Err(miette!("Use --yes to confirm").into());
                    }
                }
                state.delete_identity(identity_state)?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Identity with name '{name}' has been deleted"))
                    .machine(&name)
                    .json(serde_json::json!({ "vault": { "name": &name } }))
                    .write_line()?;
                Ok(())
            }
            // Return the appropriate error
            Err(err) => match err {
                CliStateError::NotFound => Err(miette!("Identity '{}' not found", &name).into()),
                _ => Err(err.into()),
            },
        }
    }
}
