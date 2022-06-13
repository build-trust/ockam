mod create;
mod list;
mod show;

pub(crate) use create::CreateCommand;
use list::ListCommand;
use show::ShowCommand;

use crate::{util::OckamConfig, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct TransportCommand {
    #[clap(subcommand)]
    subcommand: TransportSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TransportSubCommand {
    /// Create nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// List nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show nodes.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl TransportCommand {
    pub fn run(cfg: &mut OckamConfig, command: TransportCommand) {
        match command.subcommand {
            TransportSubCommand::Create(command) => CreateCommand::run(cfg, command),
            TransportSubCommand::List(command) => ListCommand::run(cfg, command),
            TransportSubCommand::Show(command) => ShowCommand::run(cfg, command),
        }
    }
}
