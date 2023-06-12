pub mod create;
pub mod delete;
pub mod list;
pub mod show;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

/// Manage Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct SecureChannelListenerCommand {
    #[command(subcommand)]
    subcommand: SecureChannelListenerSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SecureChannelListenerSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
    #[command(display_order = 801)]
    Delete(DeleteCommand),
    #[command(display_order = 802)]
    List(ListCommand),
    #[command(display_order = 803)]
    Show(ShowCommand),
}

impl SecureChannelListenerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SecureChannelListenerSubcommand::Create(c) => c.run(options),
            SecureChannelListenerSubcommand::Delete(c) => c.run(options),
            SecureChannelListenerSubcommand::List(c) => c.run(options),
            SecureChannelListenerSubcommand::Show(c) => c.run(options),
        }
    }
}
