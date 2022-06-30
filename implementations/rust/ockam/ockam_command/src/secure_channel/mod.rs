mod create;

pub(crate) use create::CreateCommand;

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
}

impl SecureChannelCommand {
    pub fn run(cfg: &OckamConfig, command: SecureChannelCommand) {
        match command.subcommand {
            SecureChannelSubcommand::Create(command) => CreateCommand::run(cfg, command),
        }
        .unwrap()
    }
}
