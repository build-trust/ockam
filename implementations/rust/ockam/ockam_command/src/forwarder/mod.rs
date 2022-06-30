use clap::{Args, Subcommand};

pub(crate) use create::CreateCommand;

use crate::{OckamConfig, HELP_TEMPLATE};

mod create;

#[derive(Clone, Debug, Args)]
pub struct ForwarderCommand {
    #[clap(subcommand)]
    subcommand: ForwarderSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ForwarderSubCommand {
    /// Create forwarders
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl ForwarderCommand {
    pub fn run(cfg: &OckamConfig, command: ForwarderCommand) {
        match command.subcommand {
            ForwarderSubCommand::Create(command) => CreateCommand::run(cfg, command),
        }
    }
}
