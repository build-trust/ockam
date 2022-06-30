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
pub struct ProjectCommand {
    #[clap(subcommand)]
    subcommand: ProjectSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProjectSubcommand {
    /// Create projects
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete projects
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List projects
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show projects
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl ProjectCommand {
    pub fn run(cmd: ProjectCommand) {
        match cmd.subcommand {
            ProjectSubcommand::Create(cmd) => CreateCommand::run(cmd),
            ProjectSubcommand::Delete(cmd) => DeleteCommand::run(cmd),
            ProjectSubcommand::List(cmd) => ListCommand::run(cmd),
            ProjectSubcommand::Show(cmd) => ShowCommand::run(cmd),
        }
    }
}
