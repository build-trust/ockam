use clap::{Args, Subcommand};

pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{docs, Command, CommandGlobalOpts};

mod list;
mod show;

cfg_if::cfg_if! {
    if #[cfg(feature = "admin_commands")] {
        mod create;
        mod delete;
        pub use create::CreateCommand;
        pub use delete::DeleteCommand;
    }
}

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    about = docs::about("Manage Spaces in Ockam Orchestrator"),
    long_about = docs::about(LONG_ABOUT),
)]
pub struct SpaceCommand {
    #[command(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    List(ListCommand),
    Show(ShowCommand),

    #[cfg(feature = "admin_commands")]
    Create(CreateCommand),
    #[cfg(feature = "admin_commands")]
    Delete(DeleteCommand),
}

impl SpaceCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SpaceSubcommand::List(c) => c.run(opts),
            SpaceSubcommand::Show(c) => c.run(opts),

            #[cfg(feature = "admin_commands")]
            SpaceSubcommand::Create(c) => c.run(opts),
            #[cfg(feature = "admin_commands")]
            SpaceSubcommand::Delete(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SpaceSubcommand::List(c) => c.name(),
            SpaceSubcommand::Show(c) => c.name(),

            #[cfg(feature = "admin_commands")]
            SpaceSubcommand::Create(c) => c.name(),
            #[cfg(feature = "admin_commands")]
            SpaceSubcommand::Delete(c) => c.name(),
        }
    }
}
