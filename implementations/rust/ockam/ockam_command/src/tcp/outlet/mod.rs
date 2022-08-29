mod create;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use create::CreateCommand;

/// Manage TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct TcpOutletCommand {
    #[clap(subcommand)]
    subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    Create(CreateCommand),
}

impl TcpOutletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpOutletSubCommand::Create(c) => c.run(options).unwrap(),
        }
    }
}
