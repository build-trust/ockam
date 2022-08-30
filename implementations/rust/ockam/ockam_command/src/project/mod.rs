mod add_enroller;
mod create;
mod delete;
mod delete_enroller;
mod enroll;
mod list;
mod list_enrollers;
mod show;
pub mod util;

pub use util::config;

use clap::{Args, Subcommand};

pub use crate::credential::get_credential::GetCredentialCommand;
pub use add_enroller::AddEnrollerCommand;
pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use delete_enroller::DeleteEnrollerCommand;
pub use enroll::EnrollCommand;
pub use list::ListCommand;
pub use list_enrollers::ListEnrollersCommand;
pub use show::ShowCommand;

use crate::CommandGlobalOpts;

/// Manage Projects in Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, subcommand_required = true)]
pub struct ProjectCommand {
    #[clap(subcommand)]
    subcommand: ProjectSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProjectSubcommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
    AddEnroller(AddEnrollerCommand),
    ListEnrollers(ListEnrollersCommand),
    DeleteEnroller(DeleteEnrollerCommand),
    Enroll(EnrollCommand),
}

impl ProjectCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            ProjectSubcommand::Create(c) => c.run(options),
            ProjectSubcommand::Delete(c) => c.run(options),
            ProjectSubcommand::List(c) => c.run(options),
            ProjectSubcommand::Show(c) => c.run(options),
            ProjectSubcommand::AddEnroller(c) => c.run(options),
            ProjectSubcommand::ListEnrollers(c) => c.run(options),
            ProjectSubcommand::DeleteEnroller(c) => c.run(options),
            ProjectSubcommand::Enroll(c) => c.run(options),
        }
    }
}
