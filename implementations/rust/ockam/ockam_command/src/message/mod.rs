use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
pub use send::SendCommand;

mod send;

/// Send and Receive Messages
#[derive(Clone, Debug, Args)]
pub struct MessageCommand {
    #[clap(subcommand)]
    subcommand: MessageSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum MessageSubcommand {
    /// Send messages
    #[clap(display_order = 900)]
    Send(SendCommand),
}

impl MessageCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            MessageSubcommand::Send(c) => c.run(options),
        }
    }
}
