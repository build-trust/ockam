mod get;
mod list;
mod set;

use get::GetCommand;
use list::ListCommand;
use set::SetCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct ConfigCommand {
    #[clap(subcommand)]
    subcommand: ConfigSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Set a specific configuration value
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Set(SetCommand),

    /// Get a specific configuration value
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Get(GetCommand),

    /// List all available values
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),
}

impl ConfigCommand {
    pub fn run(opts: CommandGlobalOpts, command: ConfigCommand) {
        match command.subcommand {
            ConfigSubcommand::Set(command) => SetCommand::run(opts, command),
            ConfigSubcommand::Get(command) => GetCommand::run(opts, command),
            ConfigSubcommand::List(command) => ListCommand::run(opts, command),
        }
    }
}
