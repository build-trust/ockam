mod create;
mod list;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use create::CreateCommand;
pub(crate) use list::ListCommand;

/// Manage TCP Inlets
#[derive(Clone, Debug, Args)]
pub struct TcpInletCommand {
    #[command(subcommand)]
    subcommand: TcpInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpInletSubCommand {
    Create(CreateCommand),
    List(ListCommand)
}

impl TcpInletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpInletSubCommand::Create(c) => c.run(options),
            TcpInletSubCommand::List(c) => c.run(options),
        }
    }
}
