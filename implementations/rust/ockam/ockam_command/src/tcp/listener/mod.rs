mod create;
mod delete;
mod list;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

/// Manage TCP Listeners
#[derive(Args, Clone, Debug)]
pub struct TcpListenerCommand {
    #[command(subcommand)]
    subcommand: TcpListenerSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpListenerSubCommand {
    /// Create tcp listener on the selected node
    Create(CreateCommand),

    /// Delete tcp listener on the selected node
    Delete(DeleteCommand),

    /// List tcp listeners registered on the selected node
    List(ListCommand),
}

impl TcpListenerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpListenerSubCommand::Create(c) => c.run(options),
            TcpListenerSubCommand::Delete(c) => c.run(options),
            TcpListenerSubCommand::List(c) => c.run(options),
        }
    }
}
