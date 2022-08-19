pub(crate) mod config;
pub(crate) mod start;

pub(crate) use start::StartCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct ServiceCommand {
    #[clap(subcommand)]
    subcommand: ServiceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ServiceSubcommand {
    /// Start a service
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Start(StartCommand),
}

impl ServiceCommand {
    pub fn run(opts: CommandGlobalOpts, command: ServiceCommand) {
        match command.subcommand {
            ServiceSubcommand::Start(command) => StartCommand::run(opts, command),
        }
        .unwrap()
    }
}
