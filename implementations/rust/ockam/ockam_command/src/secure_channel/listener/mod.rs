pub mod create;
pub mod list;

pub(crate) use create::CreateCommand;
pub(crate) use list::ListCommand;

use crate::secure_channel::HELP_DETAIL;
use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};

/// Manage Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[clap(
    arg_required_else_help = true,
    subcommand_required = true,
    help_template = help::template(HELP_DETAIL),
    mut_subcommand("help", |c| c.about("Print help information"))
)]
pub struct SecureChannelListenerCommand {
    #[clap(subcommand)]
    subcommand: SecureChannelListenerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelListenerSubcommand {
    #[clap(display_order = 800)]
    Create(CreateCommand),
    #[clap(display_order = 800)]
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
