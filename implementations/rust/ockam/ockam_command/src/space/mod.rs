use clap::{Args, Subcommand};

use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

use crate::HELP_TEMPLATE;

mod create;
mod delete;
mod list;
mod show;

#[derive(Clone, Debug, Args)]
pub struct SpaceCommand {
    #[clap(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    /// Create spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl SpaceCommand {
    pub fn run(command: SpaceCommand) {
        match command.subcommand {
            SpaceSubcommand::Create(command) => CreateCommand::run(command),
            SpaceSubcommand::Delete(command) => DeleteCommand::run(command),
            SpaceSubcommand::List(command) => ListCommand::run(command),
            SpaceSubcommand::Show(command) => ShowCommand::run(command),
        }
    }
}
