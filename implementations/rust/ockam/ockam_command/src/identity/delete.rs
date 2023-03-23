use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use ockam::Context;
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
) -> crate::Result<()> {
    let state = opts.state.identities;
    // Check if exists
    match state.get(&cmd.name) {
        // If it exists, proceed
        Ok(_) => {
            state.delete(&cmd.name).await?;
            println!("Identity '{}' deleted", cmd.name);
            Ok(())
        }
        // Return the appropriate error
        Err(err) => match err {
            CliStateError::NotFound(_) => Err(anyhow!("Identity '{}' not found", &cmd.name).into()),
            _ => Err(err.into()),
        },
    }
}
