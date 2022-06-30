use crate::HELP_TEMPLATE;
use clap::{Args, Subcommand};
use send::SendCommand;

mod send;

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
    pub fn run(cmd: MessageCommand) {
        match cmd.subcommand {
            MessageSubcommand::Send(cmd) => SendCommand::run(cmd),
        }
    }
}
