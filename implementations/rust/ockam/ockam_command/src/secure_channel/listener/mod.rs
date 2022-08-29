pub mod create;
pub mod list;

pub(crate) use create::CreateCommand;
pub(crate) use list::ListCommand;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

/// Manage Secure Channel Listeners
#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelListenerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelListenerSubcommand {
    Create(CreateCommand),
    List(ListCommand),
}

impl SecureChannelListenerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SecureChannelListenerSubcommand::Create(c) => c.run(options),
            SecureChannelListenerSubcommand::List(c) => c.run(options),
        }
    }
}
