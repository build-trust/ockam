use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::CommandGlobalOpts;

mod create;
mod list;
mod show;

/// Manage Relays
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct RelayCommand {
    #[command(subcommand)]
    subcommand: RelaySubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum RelaySubCommand {
    Create(CreateCommand),
    List(ListCommand),
    Show(ShowCommand),
}

impl RelayCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        match self.subcommand {
            RelaySubCommand::Create(c) => c.run(opts),
            RelaySubCommand::List(c) => c.run(opts),
            RelaySubCommand::Show(c) => c.run(opts),
        }
    }
}
