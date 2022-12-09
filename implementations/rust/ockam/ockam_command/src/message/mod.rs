use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};
pub use send::SendCommand;

mod send;

const HELP_DETAIL: &str = include_str!("../constants/message/help_detail.txt");

/// Send and Receive Messages
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = help::template(HELP_DETAIL)
)]
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
