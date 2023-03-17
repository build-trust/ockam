mod create;
mod delete;
mod list;
mod show;

use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use create::CreateCommand;
use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

const AFTER_LONG_HELP: &str = include_str!("../static/inlet/after_long_help.txt");

/// Manage TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct TcpInletCommand {
    #[command(subcommand)]
    subcommand: TcpInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpInletSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
}

impl TcpInletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpInletSubCommand::Create(c) => c.run(options),
            TcpInletSubCommand::Delete(c) => c.run(options),
            TcpInletSubCommand::List(c) => c.run(options),
            TcpInletSubCommand::Show(c) => c.run(options),
        }
    }
}
