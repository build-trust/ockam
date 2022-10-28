mod create;
mod delete;
mod list;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

/// Manage TCP Connections
#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true)]
pub struct TcpConnectionCommand {
    #[command(subcommand)]
    subcommand: TcpConnectionSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpConnectionSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
}

impl TcpConnectionCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpConnectionSubCommand::Create(c) => c.run(options),
            TcpConnectionSubCommand::Delete(c) => c.run(options),
            TcpConnectionSubCommand::List(c) => c.run(options),
        }
    }
}
