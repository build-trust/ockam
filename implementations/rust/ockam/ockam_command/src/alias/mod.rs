mod get;
mod list;
mod set;

use get::GetCommand;
use list::ListCommand;
use set::SetCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct AliasCommand {
    #[clap(subcommand)]
    subcommand: AliasSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AliasSubcommand {
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

impl AliasCommand {
    pub fn run(opts: CommandGlobalOpts, command: AliasCommand) {
        match command.subcommand {
            AliasSubcommand::Set(command) => SetCommand::run(opts, command),
            AliasSubcommand::Get(command) => GetCommand::run(opts, command),
            AliasSubcommand::List(command) => ListCommand::run(opts, command),
        }
    }
}
