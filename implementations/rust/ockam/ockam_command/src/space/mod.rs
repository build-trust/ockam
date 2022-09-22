use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;
pub use util::config;

use crate::CommandGlobalOpts;

mod create;
mod delete;
mod list;
mod show;
pub mod util;

/// Manage Spaces in Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
// TODO: mut_subcommand
//    mut_subcommand("help", |c| c.about("Print help information")),
)]
pub struct SpaceCommand {
    #[command(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    /// Create spaces
    #[command(display_order = 800)]
    Create(CreateCommand),

    /// Delete spaces
    #[command(display_order = 800)]
    Delete(DeleteCommand),

    /// List spaces
    #[command(display_order = 800)]
    List(ListCommand),

    /// Show spaces
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

pub fn random_name() -> String {
    hex::encode(&rand::random::<[u8; 4]>())
}
