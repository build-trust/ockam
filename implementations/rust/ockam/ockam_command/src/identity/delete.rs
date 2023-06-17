use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
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
    // Check if exists
    match state.identities.get(&cmd.name) {
        // If it exists, proceed
        Ok(identity_state) => {
            state.delete_identity(identity_state)?;
            println!("Identity '{}' deleted", cmd.name);
            Ok(())
        }
        // Return the appropriate error
        Err(err) => match err {
            CliStateError::NotFound => Err(miette!("Identity '{}' not found", &cmd.name)),
            _ => Err(err.into()),
        },
    }
}
