pub(crate) mod create;
pub(crate) mod delete;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;

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
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),
}

impl SecureChannelCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) {
        match command.subcommand {
            SecureChannelSubcommand::Create(sub_cmd) => sub_cmd.run(opts),
            SecureChannelSubcommand::Delete(sub_cmd) => sub_cmd.run(opts),
        }
    }
}
