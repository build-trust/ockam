use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::{docs, Command, CommandGlobalOpts};

use ockam_node::Context;

mod create;
mod delete;
mod list;
mod show;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage Relays
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct RelayCommand {
    #[command(subcommand)]
    pub subcommand: RelaySubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum RelaySubCommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
}

impl RelayCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            RelaySubCommand::Create(c) => c.run(ctx, opts).await,
            RelaySubCommand::List(c) => c.run(ctx, opts).await,
            RelaySubCommand::Show(c) => c.run(ctx, opts).await,
            RelaySubCommand::Delete(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            RelaySubCommand::Create(c) => c.name(),
            RelaySubCommand::List(c) => c.name(),
            RelaySubCommand::Show(c) => c.name(),
            RelaySubCommand::Delete(c) => c.name(),
        }
    }
}
