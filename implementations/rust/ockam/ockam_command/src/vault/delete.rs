use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::cli_state::traits::StateTrait;
use ockam_api::cli_state::CliStateError;

use crate::terminal::ConfirmResult;
use crate::util::node_rpc;
use crate::{fmt_ok, fmt_warn, CommandGlobalOpts};

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

            state.delete(&name).await?;

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
            CliStateError::NotFound => Err(anyhow!("Vault '{name}' not found").into()),
            _ => Err(err.into()),
        },
    }
}
