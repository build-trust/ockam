use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;

use crate::tcp::connection::show::ShowCommand;
use crate::{Command, CommandGlobalOpts};

mod create;
mod delete;
mod list;
mod show;

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
    Show(ShowCommand),
}

impl TcpConnectionCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            TcpConnectionSubCommand::Create(c) => c.run(opts),
            TcpConnectionSubCommand::Delete(c) => c.run(opts),
            TcpConnectionSubCommand::List(c) => c.run(opts),
            TcpConnectionSubCommand::Show(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            TcpConnectionSubCommand::Create(c) => c.name(),
            TcpConnectionSubCommand::Delete(c) => c.name(),
            TcpConnectionSubCommand::List(c) => c.name(),
            TcpConnectionSubCommand::Show(c) => c.name(),
        }
        .to_string()
    }
}
