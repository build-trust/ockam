pub(crate) mod listener;

mod create;
mod delete;
mod list;
mod show;

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{help, CommandGlobalOpts};
use clap::{Args, Subcommand};

const HELP_DETAIL: &str = include_str!("../constants/secure_channel/help_detail.txt");

/// Manage Secure Channels.
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct SecureChannelCommand {
    #[command(subcommand)]
    subcommand: SecureChannelSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum SecureChannelSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
    #[command(display_order = 800)]
    Delete(DeleteCommand),
    #[command(display_order = 800)]
    List(ListCommand),
    #[command(display_order = 800)]
    Show(ShowCommand),
}

impl SecureChannelCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SecureChannelSubcommand::Create(c) => c.run(options),
            SecureChannelSubcommand::Delete(c) => c.run(options),
            SecureChannelSubcommand::List(c) => c.run(options),
            SecureChannelSubcommand::Show(c) => c.run(options),
        }
    }
}
