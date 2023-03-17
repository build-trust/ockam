mod create;
mod delete;
mod list;
mod show;

use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use create::CreateCommand;
use delete::DeleteCommand;
use list::ListCommand;
use show::ShowCommand;

const AFTER_LONG_HELP: &str = include_str!("../static/outlet/after_long_help.txt");

/// Manage TCP Outlets
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct TcpOutletCommand {
    #[command(subcommand)]
    subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    Create(CreateCommand),
    Delete(DeleteCommand),
    List(ListCommand),
    Show(ShowCommand),
}

impl TcpOutletCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            TcpOutletSubCommand::Create(c) => c.run(options),
            TcpOutletSubCommand::Delete(c) => c.run(options),
            TcpOutletSubCommand::List(c) => c.run(options),
            TcpOutletSubCommand::Show(c) => c.run(options),
        }
    }
}
