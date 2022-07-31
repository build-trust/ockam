pub mod create;

pub(crate) use create::CreateCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelListenerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelListenerSubcommand {
    /// Create Secure Channel Listener
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl SecureChannelListenerCommand {
    pub fn run(opts: CommandGlobalOpts, command: SecureChannelListenerCommand) {
        match command.subcommand {
            SecureChannelListenerSubcommand::Create(command) => CreateCommand::run(opts, command),
        }
        .unwrap()
    }
}
