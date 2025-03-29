pub mod create;
pub mod delete;
pub mod list;
pub mod show;

pub(crate) use create::CreateCommand;
pub(crate) use delete::DeleteCommand;
pub(crate) use list::ListCommand;
pub(crate) use show::ShowCommand;

use clap::{Args, Subcommand};

use crate::CommandGlobalOpts;

use ockam_node::Context;

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
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SecureChannelListenerSubcommand::Create(c) => c.run(ctx, opts).await,
            SecureChannelListenerSubcommand::Delete(c) => c.run(ctx, opts).await,
            SecureChannelListenerSubcommand::List(c) => c.run(ctx, opts).await,
            SecureChannelListenerSubcommand::Show(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SecureChannelListenerSubcommand::Create(c) => c.name(),
            SecureChannelListenerSubcommand::Delete(c) => c.name(),
            SecureChannelListenerSubcommand::List(c) => c.name(),
            SecureChannelListenerSubcommand::Show(c) => c.name(),
        }
    }
}
