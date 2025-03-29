use clap::{Args, Subcommand};

use crate::sidecar::secure_relay_inlet::SecureRelayInlet;
use crate::sidecar::secure_relay_outlet::SecureRelayOutlet;
use crate::{docs, CommandGlobalOpts};

use ockam_node::Context;

mod secure_relay_inlet;
mod secure_relay_outlet;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");

/// Manage Sidecars
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    long_about = docs::about(LONG_ABOUT),
)]
pub struct SidecarCommand {
    #[command(subcommand)]
    pub subcommand: SidecarSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SidecarSubcommand {
    #[command(display_order = 800)]
    SecureRelayInlet(Box<SecureRelayInlet>),
    #[command(display_order = 801)]
    SecureRelayOutlet(Box<SecureRelayOutlet>),
}

impl SidecarCommand {
    pub async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match self.subcommand {
            SidecarSubcommand::SecureRelayOutlet(c) => c.run(ctx, opts).await,
            SidecarSubcommand::SecureRelayInlet(c) => c.run(ctx, opts).await,
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            SidecarSubcommand::SecureRelayInlet(c) => c.name(),
            SidecarSubcommand::SecureRelayOutlet(c) => c.name(),
        }
    }
}
