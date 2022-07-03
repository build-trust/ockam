mod create;
mod delete;
mod list;

pub(crate) use create::{CreateCommand, CreateTypeCommand};
pub(crate) use delete::DeleteCommand;
use list::ListCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct TransportCommand {
    #[clap(subcommand)]
    subcommand: TransportSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TransportSubCommand {
    /// Create transports on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete transports on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List transports registered on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),
}

impl TransportCommand {
    pub fn run(opts: CommandGlobalOpts, command: TransportCommand) {
        match command.subcommand {
            TransportSubCommand::Create(command) => CreateCommand::run(opts, command),
            TransportSubCommand::Delete(command) => DeleteCommand::run(opts, command),
            TransportSubCommand::List(command) => ListCommand::run(opts, command),
        }
    }
}
