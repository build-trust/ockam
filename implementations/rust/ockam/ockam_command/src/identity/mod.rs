mod create;
mod default;
mod delete;
mod list;
mod show;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
use ockam_api::cli_state::CliState;
pub(crate) use show::ShowCommand;

use crate::CommandGlobalOpts;
use crate::{error::Error, identity::default::DefaultCommand};
use clap::{Args, Subcommand};

/// Manage Identities
#[derive(Clone, Debug, Args)]
pub struct IdentityCommand {
    #[command(subcommand)]
    subcommand: IdentitySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum IdentitySubcommand {
    /// Create Identity
    Create(CreateCommand),
    /// Print short existing identity, `--full` for long identity
    Show(ShowCommand),
    /// Print all existing identities, `--full` for long identities
    List(ListCommand),
    /// Set the default identity
    Default(DefaultCommand),
    /// Delete an identity
    Delete(DeleteCommand),
}

impl IdentityCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            IdentitySubcommand::Create(c) => c.run(options),
            IdentitySubcommand::Show(c) => c.run(options),
            IdentitySubcommand::List(c) => c.run(options),
            IdentitySubcommand::Delete(c) => c.run(options),
            IdentitySubcommand::Default(c) => c.run(options),
        }
    }
}

pub fn default_identity_name() -> String {
    let res_cli = CliState::new();

    let cli_state = match res_cli {
        Ok(cli_state) => cli_state,
        Err(err) => {
            eprintln!("Error initializing command state. \n\n {err:?}");
            let command_err: Error = err.into();
            std::process::exit(command_err.code());
        }
    };

    let default_name = cli_state.identities.default().map(|i| i.name);

    match default_name {
        Ok(name) => name,
        Err(_) => "default".to_string(),
    }
}
