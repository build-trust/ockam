mod create;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use create::CreateCommand;

/// Manage TCP Inlets
#[derive(Clone, Debug, Args)]
pub struct TcpInletCommand {
    #[command(subcommand)]
    subcommand: TcpInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpInletSubCommand {
    Create(CreateCommand),
}

impl TcpInletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpInletSubCommand::Create(c) => c.run(options).unwrap(),
        }
    }
}
