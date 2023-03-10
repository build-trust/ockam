mod create;
mod delete;
mod list;
mod show;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

/// Manage TCP Outlets
#[derive(Clone, Debug, Args)]
pub struct TcpOutletCommand {
    #[command(subcommand)]
    subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    Show(ShowCommand),
    List(ListCommand),
}

impl TcpOutletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpOutletSubCommand::Create(c) => c.run(options),
            TcpOutletSubCommand::Delete(c) => c.run(options),
            TcpOutletSubCommand::Show(c) => c.run(options),
            TcpOutletSubCommand::List(c) => c.run(options),
        }
    }
}
