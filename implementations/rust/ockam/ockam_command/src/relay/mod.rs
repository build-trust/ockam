use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::{docs, CommandGlobalOpts};

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
    subcommand: RelaySubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum RelaySubCommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
    Delete(DeleteCommand),
}

impl RelayCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            RelaySubCommand::Create(c) => c.run(opts),
            RelaySubCommand::List(c) => c.run(opts),
            RelaySubCommand::Show(c) => c.run(opts),
            RelaySubCommand::Delete(c) => c.run(opts),
        }
    }
}
