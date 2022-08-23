mod add_enroller;
mod add_member;
mod create;
mod delete;
mod delete_enroller;
mod get_credential;
mod list;
mod list_enrollers;
mod show;
pub mod util;

use clap::{Args, Subcommand};

pub use add_enroller::AddEnrollerCommand;
pub use add_member::AddMemberCommand;
pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use delete_enroller::DeleteEnrollerCommand;
pub use get_credential::GetCredentialCommand;
pub use list::ListCommand;
pub use list_enrollers::ListEnrollersCommand;
pub use show::ShowCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};

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

    /// An authorised enroller can add members to a project.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    AddMember(AddMemberCommand),

    /// An authorised member can request a credential from the projects's authority.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    GetCredential(GetCredentialCommand),
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
            ProjectSubcommand::AddMember(scmd) => scmd.run(opts),
            ProjectSubcommand::GetCredential(scmd) => scmd.run(opts),
        }
    }
}
