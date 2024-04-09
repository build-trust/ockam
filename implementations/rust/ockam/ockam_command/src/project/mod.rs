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

use crate::{docs, Command, CommandGlobalOpts};

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
    pub subcommand: ProjectSubcommand,
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ProjectSubcommand::Create(c) => c.run(opts),
            ProjectSubcommand::Import(c) => c.run(opts),
            ProjectSubcommand::Delete(c) => c.run(opts),
            ProjectSubcommand::List(c) => c.run(opts),
            ProjectSubcommand::Show(c) => c.run(opts),
            ProjectSubcommand::Version(c) => c.run(opts),
            ProjectSubcommand::Ticket(c) => c.run(opts),
            ProjectSubcommand::Information(c) => c.run(opts),
            ProjectSubcommand::Addon(c) => c.run(opts),
            ProjectSubcommand::Enroll(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ProjectSubcommand::Create(c) => c.name(),
            ProjectSubcommand::Delete(c) => c.name(),
            ProjectSubcommand::List(c) => c.name(),
            ProjectSubcommand::Show(c) => c.name(),
            ProjectSubcommand::Import(c) => c.name(),
            ProjectSubcommand::Version(c) => c.name(),
            ProjectSubcommand::Information(c) => c.name(),
            ProjectSubcommand::Ticket(c) => c.name(),
            ProjectSubcommand::Addon(c) => c.name(),
            ProjectSubcommand::Enroll(c) => c.name(),
        }
    }
}
