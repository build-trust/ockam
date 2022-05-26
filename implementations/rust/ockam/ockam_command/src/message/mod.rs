mod send;

use clap::{Args, Subcommand};
use send::SendCommand;

#[derive(Clone, Debug, Args)]
pub struct MessageCommand {
    #[clap(subcommand)]
    subcommand: MessageSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum MessageSubcommand {
    /// Send messages.
    Send(SendCommand),
}

impl MessageCommand {
    pub fn run(command: MessageCommand) {
        match command.subcommand {
            MessageSubcommand::Send(command) => SendCommand::run(command),
        }
    }
}
