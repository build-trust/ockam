mod send;

use crate::HELP_TEMPLATE;
use clap::{Args, Subcommand};
use send::SendCommand;

#[derive(Clone, Debug, Args)]
pub struct MessageCommand {
    #[clap(subcommand)]
    subcommand: MessageSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum MessageSubcommand {
    /// Send messages
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Send(SendCommand),
}

impl MessageCommand {
    pub fn run(command: MessageCommand) {
        match command.subcommand {
            MessageSubcommand::Send(command) => SendCommand::run(command),
        }
    }
}
