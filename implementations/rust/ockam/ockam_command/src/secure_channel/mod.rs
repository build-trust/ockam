mod create;
mod create_listener;

pub(crate) use create::CreateCommand;
pub(crate) use create_listener::CreateListenerCommand;

use crate::{util::OckamConfig, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct SecureChannelCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelSubcommand {
    /// Create Secure Channel
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Create Secure Channel Listener
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    CreateListener(CreateListenerCommand),
}

impl SecureChannelCommand {
    pub fn run(cfg: &mut OckamConfig, command: SecureChannelCommand) {
        match command.subcommand {
            SecureChannelSubcommand::Create(command) => CreateCommand::run(cfg, command),
            SecureChannelSubcommand::CreateListener(command) => {
                CreateListenerCommand::run(cfg, command)
            }
        }
        .unwrap()
    }
}
