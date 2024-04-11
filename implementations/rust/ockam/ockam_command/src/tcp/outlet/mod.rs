use clap::{Args, Subcommand};

use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

use crate::{docs, Command, CommandGlobalOpts};

pub mod create;
mod delete;
pub mod list;
mod show;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage TCP Outlets
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct TcpOutletCommand {
    #[command(subcommand)]
    pub subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
}

impl TcpOutletCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            TcpOutletSubCommand::Create(c) => c.run(opts),
            TcpOutletSubCommand::Delete(c) => c.run(opts),
            TcpOutletSubCommand::List(c) => c.run(opts),
            TcpOutletSubCommand::Show(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            TcpOutletSubCommand::Create(c) => c.name(),
            TcpOutletSubCommand::Delete(c) => c.name(),
            TcpOutletSubCommand::List(c) => c.name(),
            TcpOutletSubCommand::Show(c) => c.name(),
        }
    }
}
