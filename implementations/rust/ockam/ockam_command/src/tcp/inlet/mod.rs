mod create;
mod delete;
mod list;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;

/// Manage TCP Inlets
#[derive(Clone, Debug, Args)]
pub struct TcpInletCommand {
    #[clap(subcommand)]
    subcommand: TcpInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpInletSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl TcpInletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpInletSubCommand::Create(c) => c.run(options),
            TcpInletSubCommand::Delete(c) => c.run(options),
            TcpInletSubCommand::List(c) => c.run(options),
        }
    }
}
