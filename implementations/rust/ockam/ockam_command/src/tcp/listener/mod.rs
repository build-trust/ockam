mod create;
mod delete;
mod list;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Args, Clone, Debug)]
pub struct TcpListenerCommand {
    #[clap(subcommand)]
    subcommand: TcpListenerSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpListenerSubCommand {
    /// Create tcp listener on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete tcp listener on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List tcp listeners registered on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),
}

impl TcpListenerCommand {
    pub fn run(opts: CommandGlobalOpts, command: TcpListenerCommand) {
        match command.subcommand {
            TcpListenerSubCommand::Create(command) => CreateCommand::run(opts, command),
            TcpListenerSubCommand::Delete(command) => DeleteCommand::run(opts, command),
            TcpListenerSubCommand::List(command) => ListCommand::run(opts, command),
        }
    }
}
