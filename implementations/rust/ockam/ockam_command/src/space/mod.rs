use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};

mod create;
mod delete;
mod list;
mod show;

#[derive(Clone, Debug, Args)]
pub struct SpaceCommand {
    #[clap(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    /// Create spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl SpaceCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: SpaceCommand) {
        match cmd.subcommand {
            SpaceSubcommand::Create(cmd) => CreateCommand::run(opts, cmd),
            SpaceSubcommand::Delete(cmd) => DeleteCommand::run(opts, cmd),
            SpaceSubcommand::List(cmd) => ListCommand::run(opts, cmd),
            SpaceSubcommand::Show(cmd) => ShowCommand::run(opts, cmd),
        }
    }
}
