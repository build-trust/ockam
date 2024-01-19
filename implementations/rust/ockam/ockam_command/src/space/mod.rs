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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SpaceSubcommand::Create(c) => c.run(opts),
            SpaceSubcommand::Delete(c) => c.run(opts),
            SpaceSubcommand::List(c) => c.run(opts),
            SpaceSubcommand::Show(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SpaceSubcommand::Create(c) => c.name(),
            SpaceSubcommand::Delete(c) => c.name(),
            SpaceSubcommand::List(c) => c.name(),
            SpaceSubcommand::Show(c) => c.name(),
        }
    }
}
