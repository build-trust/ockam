use clap::{Args, Subcommand};

pub use add_enroller::AddEnrollerCommand;
pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use delete_enroller::DeleteEnrollerCommand;
pub use list::ListCommand;
pub use list_enrollers::ListEnrollersCommand;
pub use show::ShowCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};

mod add_enroller;
mod create;
mod delete;
mod delete_enroller;
mod list;
mod list_enrollers;
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

    /// Adds an authorized enroller to the project' authority
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    AddEnroller(AddEnrollerCommand),

    /// List a project' authority authorized enrollers
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    ListEnrollers(ListEnrollersCommand),

    /// Remove an identity as authorized enroller from the project' authority
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    DeleteEnroller(DeleteEnrollerCommand),
}

impl ProjectCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: ProjectCommand) {
        match cmd.subcommand {
            ProjectSubcommand::Create(scmd) => CreateCommand::run(opts, scmd),
            ProjectSubcommand::Delete(scmd) => DeleteCommand::run(opts, scmd),
            ProjectSubcommand::List(scmd) => ListCommand::run(opts, scmd),
            ProjectSubcommand::Show(scmd) => ShowCommand::run(opts, scmd),
            ProjectSubcommand::AddEnroller(scmd) => AddEnrollerCommand::run(opts, scmd),
            ProjectSubcommand::ListEnrollers(scmd) => ListEnrollersCommand::run(opts, scmd),
            ProjectSubcommand::DeleteEnroller(scmd) => DeleteEnrollerCommand::run(opts, scmd),
        }
    }
}
