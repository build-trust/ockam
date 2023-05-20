mod addon;
mod authenticate;
mod create;
mod delete;
mod info;
mod list;
mod show;
mod ticket;
pub mod util;

pub use info::ProjectInfo;
pub use util::config;

use clap::{Args, Subcommand};

pub use crate::credential::get::GetCommand;
pub use addon::AddonCommand;
pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use info::InfoCommand;
pub use list::ListCommand;
pub use show::ShowCommand;
pub use ticket::TicketCommand;

use crate::docs;
use crate::project::authenticate::AuthenticateCommand;
use crate::CommandGlobalOpts;

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
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
    Information(InfoCommand),
    Ticket(TicketCommand),
    Addon(AddonCommand),
    Authenticate(AuthenticateCommand),
}

impl ProjectCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            ProjectSubcommand::Create(c) => c.run(options),
            ProjectSubcommand::Delete(c) => c.run(options),
            ProjectSubcommand::List(c) => c.run(options),
            ProjectSubcommand::Show(c) => c.run(options),
            ProjectSubcommand::Ticket(c) => c.run(options),
            ProjectSubcommand::Information(c) => c.run(options),
            ProjectSubcommand::Addon(c) => c.run(options),
            ProjectSubcommand::Authenticate(c) => c.run(options),
        }
    }
}
