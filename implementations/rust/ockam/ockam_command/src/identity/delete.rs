use crate::util::node_rpc;
use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;
use ockam::Context;
use ockam_api::cli_state::CliStateError;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
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
