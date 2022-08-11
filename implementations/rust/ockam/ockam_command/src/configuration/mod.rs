mod get;
mod list;
mod set;

use get::GetCommand;
use list::ListCommand;
use set::SetCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct ConfigurationCommand {
    #[clap(subcommand)]
    subcommand: ConfigurationSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ConfigurationSubcommand {
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

impl ConfigurationCommand {
    pub fn run(opts: CommandGlobalOpts, command: ConfigurationCommand) {
        match command.subcommand {
            ConfigurationSubcommand::Set(command) => SetCommand::run(opts, command),
            ConfigurationSubcommand::Get(command) => GetCommand::run(opts, command),
            ConfigurationSubcommand::List(command) => ListCommand::run(opts, command),
        }
    }
}
