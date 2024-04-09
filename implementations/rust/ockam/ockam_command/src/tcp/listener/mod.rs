use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::CommandGlobalOpts;

mod create;
mod delete;
mod list;
mod show;

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

    /// Show tcp listener details
    Show(ShowCommand),
}

impl TcpListenerCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            TcpListenerSubCommand::Create(c) => c.run(opts),
            TcpListenerSubCommand::Delete(c) => c.run(opts),
            TcpListenerSubCommand::List(c) => c.run(opts),
            TcpListenerSubCommand::Show(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            TcpListenerSubCommand::Create(c) => c.name(),
            TcpListenerSubCommand::Delete(c) => c.name(),
            TcpListenerSubCommand::List(c) => c.name(),
            TcpListenerSubCommand::Show(c) => c.name(),
        }
    }
}
