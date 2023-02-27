mod create;
mod list;
mod delete;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use create::CreateCommand;
use list::ListCommand;
use delete::DeleteCommand;


/// Manage TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct TcpOutletCommand {
    #[command(subcommand)]
    subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    Create(CreateCommand),
    List(ListCommand),
    Delete(DeleteCommand),
}

impl TcpOutletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpOutletSubCommand::Create(c) => c.run(options),
            TcpOutletSubCommand::List(c) => c.run(options),
            TcpOutletSubCommand::Delete(c) => c.run(options),
        }
    }
}
