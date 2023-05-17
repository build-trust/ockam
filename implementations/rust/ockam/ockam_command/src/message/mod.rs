use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
pub use send::SendCommand;

mod send;

/// Send and receive messages
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct MessageCommand {
    #[command(subcommand)]
    subcommand: MessageSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum MessageSubcommand {
    #[command(display_order = 800)]
    Send(SendCommand),
}

impl MessageCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            MessageSubcommand::Send(c) => c.run(options),
        }
    }
}
