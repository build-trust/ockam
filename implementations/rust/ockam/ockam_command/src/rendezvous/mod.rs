mod create;
mod get_my_address;

use clap::{Args, Subcommand};

use create::CreateCommand;

use crate::rendezvous::get_my_address::GetMyAddressCommand;
use crate::{docs, Command, CommandGlobalOpts};

use ockam_node::Context;

/// Manage Rendezvous server
#[derive(Clone, Debug, Args)]
#[command(hide = docs::hide(), arg_required_else_help = true, subcommand_required = true)]
pub struct RendezvousCommand {
    #[command(subcommand)]
    pub subcommand: RendezvousSubcommand,
}

impl RendezvousCommand {
    pub fn name(&self) -> String {
        self.subcommand.name()
    }
}

#[derive(Clone, Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum RendezvousSubcommand {
    #[command(display_order = 800)]
    Create(CreateCommand),
    #[command(display_order = 800)]
    GetMyAddress(GetMyAddressCommand),
}

impl RendezvousSubcommand {
    pub fn name(&self) -> String {
        match self {
            RendezvousSubcommand::Create(c) => c.name(),
            RendezvousSubcommand::GetMyAddress(c) => c.name(),
        }
    }
}

impl RendezvousCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            RendezvousSubcommand::Create(c) => c.run(ctx, opts).await,
            RendezvousSubcommand::GetMyAddress(c) => c.run(ctx, opts).await,
        }
    }
}
