mod create;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};
use create::CreateCommand;

#[derive(Clone, Debug, Args)]
pub struct TcpInletCommand {
    #[clap(subcommand)]
    subcommand: TcpInletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpInletSubCommand {
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl TcpInletCommand {
    pub fn run(options: CommandGlobalOpts, command: TcpInletCommand) {
        match command.subcommand {
            TcpInletSubCommand::Create(command) => CreateCommand::run(options, command).unwrap(),
        }
    }
}
