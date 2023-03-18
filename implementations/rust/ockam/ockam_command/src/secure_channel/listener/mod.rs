mod common;
pub mod create;
pub mod delete;
pub mod list;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;

use crate::secure_channel::HELP_DETAIL;
use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};

/// Manage Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct SecureChannelListenerCommand {
    #[command(subcommand)]
    subcommand: SecureChannelListenerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelListenerSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
    #[command(display_order = 800)]
    List(ListCommand),
}

impl SecureChannelListenerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SecureChannelListenerSubcommand::Create(c) => c.run(options),
            SecureChannelListenerSubcommand::Delete(c) => c.run(options),
            SecureChannelListenerSubcommand::List(c) => c.run(options),
        }
    }
}
