mod create;
mod delete;
mod list;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct TcpConnectionCommand {
    #[clap(subcommand)]
    subcommand: TcpConnectionSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpConnectionSubCommand {
    /// Create tcp connection on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete tcp connection on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),
    /// List tcp connections registered on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),
}

impl TcpConnectionCommand {
    pub fn run(opts: CommandGlobalOpts, command: TcpConnectionCommand) {
        match command.subcommand {
            TcpConnectionSubCommand::Create(command) => CreateCommand::run(opts, command),
            TcpConnectionSubCommand::Delete(command) => DeleteCommand::run(opts, command),
            TcpConnectionSubCommand::List(command) => ListCommand::run(opts, command),
        }
    }
}
