use clap::{Args, Subcommand};

pub use send::SendCommand;

use crate::{Command, CommandGlobalOpts};

use ockam_node::Context;

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
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            MessageSubcommand::Send(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            MessageSubcommand::Send(c) => c.name(),
        }
    }
}
