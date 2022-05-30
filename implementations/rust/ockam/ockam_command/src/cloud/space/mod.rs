mod create;
mod delete;
mod list;
mod show;

use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct SpaceCommand {
    #[clap(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    /// Create spaces
    #[clap(display_order = 900)]
    Create(CreateCommand),

    /// Delete spaces
    #[clap(display_order = 900)]
    Delete(DeleteCommand),

    /// List spaces
    #[clap(display_order = 900)]
    List(ListCommand),

    /// Show spaces
    #[clap(display_order = 900)]
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
