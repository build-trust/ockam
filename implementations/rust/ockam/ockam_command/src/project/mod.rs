use crate::docs;
use clap::{Args, Subcommand};

pub use addon::AddonCommand;
pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use enroll::EnrollCommand;
pub use import::ImportCommand;
pub use info::InfoCommand;
pub use list::ListCommand;
pub use show::ShowCommand;
pub use ticket::TicketCommand;
pub use version::VersionCommand;

use crate::CommandGlobalOpts;

mod addon;
mod create;
mod delete;
pub(crate) mod enroll;
mod import;
mod info;
mod list;
mod show;
mod ticket;
pub mod util;
mod version;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Projects in Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
subcommand_required = true,
long_about = docs::about(LONG_ABOUT),
)]
pub struct ProjectCommand {
    #[command(subcommand)]
    subcommand: ProjectSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProjectSubcommand {
    Create(CreateCommand),
    Import(ImportCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
    Version(VersionCommand),
    Information(InfoCommand),
    Ticket(TicketCommand),
    Addon(AddonCommand),
    Enroll(Box<EnrollCommand>),
}

impl ProjectCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            ProjectSubcommand::Create(c) => c.run(options),
            ProjectSubcommand::Import(c) => c.run(options),
            ProjectSubcommand::Delete(c) => c.run(options),
            ProjectSubcommand::List(c) => c.run(options),
            ProjectSubcommand::Show(c) => c.run(options),
            ProjectSubcommand::Version(c) => c.run(options),
            ProjectSubcommand::Ticket(c) => c.run(options),
            ProjectSubcommand::Information(c) => c.run(options),
            ProjectSubcommand::Addon(c) => c.run(options),
            ProjectSubcommand::Enroll(c) => c.run(options),
        }
    }
}
