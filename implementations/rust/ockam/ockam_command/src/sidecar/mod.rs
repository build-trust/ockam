use crate::sidecar::secure_relay_inlet::SecureRelayInlet;
use crate::{docs, CommandGlobalOpts};
use clap::{Args, Subcommand};

mod secure_relay_inlet;
mod secure_relay_outlet;
use crate::sidecar::secure_relay_outlet::SecureRelayOutlet;

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
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            SidecarSubcommand::SecureRelayOutlet(c) => c.run(options),
            SidecarSubcommand::SecureRelayInlet(c) => c.run(options),
        }
    }
}
