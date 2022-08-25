mod create;
mod list;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use create::CreateCommand;
use list::ListCommand;

/// Manage TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct TcpOutletCommand {
    #[clap(subcommand)]
    subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    Create(CreateCommand),
    List(ListCommand),
}

impl TcpOutletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpOutletSubCommand::Create(c) => c.run(options),
            TcpOutletSubCommand::List(c) => c.run(options),
        }
    }
}
