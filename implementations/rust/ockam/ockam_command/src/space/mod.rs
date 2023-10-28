use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{docs, CommandGlobalOpts};

mod create;
mod delete;
mod list;
mod show;
mod util;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Spaces in Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct SpaceCommand {
    #[command(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
    #[command(display_order = 800)]
    List(ListCommand),
    #[command(display_order = 800)]
    Show(ShowCommand),
}

impl SpaceCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SpaceSubcommand::Create(c) => c.run(options),
            SpaceSubcommand::Delete(c) => c.run(options),
            SpaceSubcommand::List(c) => c.run(options),
            SpaceSubcommand::Show(c) => c.run(options),
        }
    }
}
