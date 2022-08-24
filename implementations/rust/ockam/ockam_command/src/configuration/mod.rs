mod get;
mod get_default_node;
mod list;
mod set;
mod set_default_node;

use get::GetCommand;
use get_default_node::GetDefaultNodeCommand;
use list::ListCommand;
use set::SetCommand;
use set_default_node::SetDefaultNodeCommand;

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

    /// Set Default Node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    SetDefaultNode(SetDefaultNodeCommand),

    /// Get Default Node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    GetDefaultNode(GetDefaultNodeCommand),
}

impl ConfigurationCommand {
    pub fn run(opts: CommandGlobalOpts, command: ConfigurationCommand) {
        match command.subcommand {
            ConfigurationSubcommand::Set(command) => SetCommand::run(opts, command),
            ConfigurationSubcommand::Get(command) => GetCommand::run(opts, command),
            ConfigurationSubcommand::List(command) => ListCommand::run(opts, command),
            ConfigurationSubcommand::SetDefaultNode(command) => {
                SetDefaultNodeCommand::run(opts, command)
            }
            ConfigurationSubcommand::GetDefaultNode(command) => {
                GetDefaultNodeCommand::run(opts, command)
            }
        }
    }
}
