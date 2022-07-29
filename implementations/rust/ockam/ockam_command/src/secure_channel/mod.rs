pub(crate) mod create;

pub(crate) use create::CreateCommand;
pub(crate) use create::CreateListenerCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct SecureChannelCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelSubcommand {
    /// Create Secure Channel Connector
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Create Secure Channel Listener
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    CreateListener(CreateListenerCommand),
}

impl SecureChannelCommand {
    pub fn run(opts: CommandGlobalOpts, command: SecureChannelCommand) {
        match command.subcommand {
            SecureChannelSubcommand::Create(command) => CreateCommand::run(opts, command),
            SecureChannelSubcommand::CreateListener(command) => {
                CreateListenerCommand::run(opts, command)
            }
        }
        .unwrap()
    }
}
