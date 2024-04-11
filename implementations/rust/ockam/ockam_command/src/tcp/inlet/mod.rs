use clap::{Args, Subcommand};

use create::CreateCommand;
use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::{docs, Command, CommandGlobalOpts};

pub(crate) mod create;
mod delete;
mod list;
mod show;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct TcpInletCommand {
    #[command(subcommand)]
    pub subcommand: TcpInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpInletSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
}

impl TcpInletCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            TcpInletSubCommand::Create(c) => c.run(opts),
            TcpInletSubCommand::Delete(c) => c.run(opts),
            TcpInletSubCommand::List(c) => c.run(opts),
            TcpInletSubCommand::Show(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            TcpInletSubCommand::Create(c) => c.name(),
            TcpInletSubCommand::Delete(c) => c.name(),
            TcpInletSubCommand::List(c) => c.name(),
            TcpInletSubCommand::Show(c) => c.name(),
        }
    }
}
