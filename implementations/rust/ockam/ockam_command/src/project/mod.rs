use clap::{Args, Subcommand};

pub use enroll::EnrollCommand;
pub use import::ImportCommand;
pub use info::InfoCommand;
pub use list::ListCommand;
pub use show::ShowCommand;
pub use version::VersionCommand;

use crate::{docs, Command, CommandGlobalOpts};

pub(crate) mod enroll;
mod import;
mod info;
mod list;
mod show;
#[allow(unused)]
pub mod util;
mod version;

cfg_if::cfg_if! {
    if #[cfg(feature = "admin_commands")] {
        mod addon;
        mod create;
        mod delete;
        mod ticket;
        pub use addon::AddonCommand;
        pub use create::CreateCommand;
        pub use delete::DeleteCommand;
        pub use ticket::TicketCommand;
    }
}

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
subcommand_required = true,
about = docs::about("Manage Projects in Ockam Orchestrator"),
long_about = docs::about(LONG_ABOUT),
)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub subcommand: ProjectSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProjectSubcommand {
    Enroll(EnrollCommand),
    Import(ImportCommand),
    List(ListCommand),
    Show(ShowCommand),
    Version(VersionCommand),
    Information(InfoCommand),

    #[cfg(feature = "admin_commands")]
    Ticket(TicketCommand),
    #[cfg(feature = "admin_commands")]
    Create(CreateCommand),
    #[cfg(feature = "admin_commands")]
    Delete(DeleteCommand),
    #[cfg(feature = "admin_commands")]
    Addon(AddonCommand),
}

impl ProjectCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            ProjectSubcommand::Enroll(c) => c.run(opts),
            ProjectSubcommand::Import(c) => c.run(opts),
            ProjectSubcommand::List(c) => c.run(opts),
            ProjectSubcommand::Show(c) => c.run(opts),
            ProjectSubcommand::Version(c) => c.run(opts),
            ProjectSubcommand::Information(c) => c.run(opts),

            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Ticket(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Create(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Delete(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Addon(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ProjectSubcommand::Enroll(c) => c.name(),
            ProjectSubcommand::Import(c) => c.name(),
            ProjectSubcommand::List(c) => c.name(),
            ProjectSubcommand::Show(c) => c.name(),
            ProjectSubcommand::Version(c) => c.name(),
            ProjectSubcommand::Information(c) => c.name(),

            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Ticket(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Create(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Delete(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            ProjectSubcommand::Addon(c) => c.name(),
        }
    }
}
