mod attach_key;
mod create;
mod default;
mod delete;
mod list;
mod show;

use crate::error::Error;
use crate::vault::attach_key::AttachKeyCommand;
use crate::vault::create::CreateCommand;
use crate::vault::default::DefaultCommand;
use crate::vault::delete::DeleteCommand;
use crate::vault::list::ListCommand;
use crate::vault::show::ShowCommand;
use crate::CommandGlobalOpts;

use clap::{Args, Subcommand};
use ockam_api::cli_state::traits::{StateItemDirTrait, StateTrait};
use ockam_api::cli_state::CliState;

/// Manage vaults
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct VaultCommand {
    #[command(subcommand)]
    subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    /// Create a vault
    Create(CreateCommand),
    /// Attach a key to a vault
    #[command(arg_required_else_help = true)]
    AttachKey(AttachKeyCommand),
    /// Show vault details
    Show(ShowCommand),
    /// Delete a vault
    Delete(DeleteCommand),
    /// List vaults
    List(ListCommand),
    /// Set the default identity
    Default(DefaultCommand),
}

impl VaultCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            VaultSubcommand::Create(cmd) => cmd.run(opts),
            VaultSubcommand::AttachKey(cmd) => cmd.run(opts),
            VaultSubcommand::Show(cmd) => cmd.run(opts),
            VaultSubcommand::List(cmd) => cmd.run(opts),
            VaultSubcommand::Delete(cmd) => cmd.run(opts),
            VaultSubcommand::Default(cmd) => cmd.run(opts),
        }
    }
}

pub fn default_vault_name() -> String {
    let res_cli = CliState::try_default();

    let cli_state = match res_cli {
        Ok(cli_state) => cli_state,
        Err(err) => {
            eprintln!("Error initializing command state. \n\n {err:?}");
            let command_err: Error = err.into();
            std::process::exit(command_err.code());
        }
    };

    cli_state
        .vaults
        .default()
        .map_or("default".to_string(), |v| v.name().to_string())
}
