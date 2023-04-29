use crate::{fmt_ok, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::traits::{StateItemDirTrait, StateTrait};
use ockam_api::cli_state::CliStateError;

/// Set the default vault
#[derive(Clone, Debug, Args)]
pub struct DefaultCommand {
    /// Name of the vault to be set as default
    name: String,
}

impl DefaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if let Err(e) = run_impl(opts, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DefaultCommand) -> crate::Result<()> {
    let DefaultCommand { name } = cmd;
    let state = opts.state.vaults;
    match state.get(&name) {
        Ok(v) => {
            // If it exists, warn the user and exit
            if state.is_default(v.name())? {
                Err(anyhow!("Vault '{name}' is already the default").into())
            }
            // Otherwise, set it as default
            else {
                state.set_default(v.name())?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("Vault '{name}' is now the default"))
                    .machine(&name)
                    .json(serde_json::json!({ "vault": {"name": name} }))
                    .write_line()?;
                Ok(())
            }
        }
        Err(err) => match err {
            CliStateError::NotFound => Err(anyhow!("Vault '{name}' not found").into()),
            _ => Err(err.into()),
        },
    }
}
