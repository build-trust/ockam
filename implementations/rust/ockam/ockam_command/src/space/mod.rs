use clap::{Args, Subcommand};

pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{docs, Command, CommandGlobalOpts};

mod create;
mod delete;
mod list;
mod show;

pub use create::CreateCommand;
pub use delete::DeleteCommand;

use ockam_node::Context;

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
    Create(CreateCommand),
    Delete(DeleteCommand),
}

impl SpaceCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SpaceSubcommand::List(c) => c.run(ctx, opts).await,
            SpaceSubcommand::Show(c) => c.run(ctx, opts).await,
            SpaceSubcommand::Create(c) => c.run(ctx, opts).await,
            SpaceSubcommand::Delete(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SpaceSubcommand::List(c) => c.name(),
            SpaceSubcommand::Show(c) => c.name(),
            SpaceSubcommand::Create(c) => c.name(),
            SpaceSubcommand::Delete(c) => c.name(),
        }
    }
}
