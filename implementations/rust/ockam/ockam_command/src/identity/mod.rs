mod create;
mod delete;
mod list;
mod show;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::CommandGlobalOpts;
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
    Delete(DeleteCommand),
}

impl IdentityCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            IdentitySubcommand::Create(c) => c.run(options),
            IdentitySubcommand::Show(c) => c.run(options),
            IdentitySubcommand::List(c) => c.run(options),
            IdentitySubcommand::Delete(c) => c.run(options),
        }
    }
}
