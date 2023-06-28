use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, fmt_ok};
use clap::Args;
use miette::miette;
use colorful::Colorful;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use crate::terminal::ConfirmResult;

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
) -> miette::Result<()> {
    let state = opts.state;
    let idt = state.identities.get(&cmd.name)?;
    // Check if --yes flag is provided
    if cmd.yes {
        // check if exists
        state.delete_identity(idt)?;
    } else {
        // If yes is not provided make sure using TTY
        // If it exists, proceed
        match opts.terminal.confirm("This will delete the selected Identity. Are you sure?")? {
            ConfirmResult::Yes => {}
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(miette!("Use --yes to confirm").into());
            }
        }
        state.delete_identity(idt)?;
    }
    // print message
    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "The identity named '{}' has been deleted.",
            &cmd.name
        ))
        .machine(&cmd.name)
        .json(serde_json::json!({ "name": &cmd.name }))
        .write_line()?;
    Ok(())
}
