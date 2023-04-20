use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::CliStateError;

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
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
    // Check if exists
    match state.get(&name) {
        // If it exists, proceed
        Ok(_) => {
            state.delete(&name).await?;

            // log the deletion
            opts.shell
                .stdout()
                .plain(format!(
                    "{} Vault with name '{}' has been deleted.",
                    "✔︎".light_green(),
                    &name
                ))
                .machine(&name)
                .json(&serde_json::json!({ "vault": { "name": &name } }))
                .write_line()?;

            Ok(())
        }
        // Return the appropriate error
        Err(err) => match err {
            CliStateError::NotFound(_) => Err(anyhow!("Vault '{name}' not found").into()),
            _ => Err(err.into()),
        },
    }
}
