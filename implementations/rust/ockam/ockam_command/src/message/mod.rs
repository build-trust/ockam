use clap::{Args, Subcommand};

pub use send::SendCommand;

use crate::CommandGlobalOpts;

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            MessageSubcommand::Send(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            MessageSubcommand::Send(c) => c.name(),
        }
    }
}
