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
pub struct ProjectCommand {
    #[clap(subcommand)]
    subcommand: ProjectSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProjectSubcommand {
    /// Create projects
    #[clap(display_order = 900)]
    Create(CreateCommand),

    /// Delete projects
    #[clap(display_order = 900)]
    Delete(DeleteCommand),

    /// List projects
    #[clap(display_order = 900)]
    List(ListCommand),

    /// Show projects
    #[clap(display_order = 900)]
    Show(ShowCommand),
}

impl ProjectCommand {
    pub fn run(command: ProjectCommand) {
        match command.subcommand {
            ProjectSubcommand::Create(command) => CreateCommand::run(command),
            ProjectSubcommand::Delete(command) => DeleteCommand::run(command),
            ProjectSubcommand::List(command) => ListCommand::run(command),
            ProjectSubcommand::Show(command) => ShowCommand::run(command),
        }
    }
}
