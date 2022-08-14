mod create;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};
use create::CreateCommand;

#[derive(Clone, Debug, Args)]
pub struct TcpOutletCommand {
    #[clap(subcommand)]
    subcommand: TcpOutletSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum TcpOutletSubCommand {
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl TcpOutletCommand {
    pub fn run(options: CommandGlobalOpts, command: TcpOutletCommand) {
        match command.subcommand {
            TcpOutletSubCommand::Create(command) => CreateCommand::run(options, command).unwrap(),
        }
    }
}
