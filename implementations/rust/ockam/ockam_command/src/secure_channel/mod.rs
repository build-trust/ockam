pub(crate) mod listener;

mod create;
mod delete;
mod list;
mod show;

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
pub use show::ShowCommand;

use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Manage Secure Channels.
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
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
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SecureChannelSubcommand::Create(c) => c.run(ctx, opts).await,
            SecureChannelSubcommand::Delete(c) => c.run(ctx, opts).await,
            SecureChannelSubcommand::List(c) => c.run(ctx, opts).await,
            SecureChannelSubcommand::Show(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SecureChannelSubcommand::Create(c) => c.name(),
            SecureChannelSubcommand::Delete(c) => c.name(),
            SecureChannelSubcommand::List(c) => c.name(),
            SecureChannelSubcommand::Show(c) => c.name(),
        }
    }
}
