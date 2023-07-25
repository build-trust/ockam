use clap::{Args, Subcommand};

use crate::CommandGlobalOpts;

mod list;
mod output;
mod service;

pub use list::ListCommand;
pub use service::ServiceCreateCommand;

/// Manage sharing invitations in Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct ShareCommand {
    #[command(subcommand)]
    subcommand: ShareSubcommand,
}

// This can be removed when the other variants each become wrappers
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Subcommand)]
pub enum ShareSubcommand {
    /// Accept a received sharing invitation
    Accept,
    /// Create an invitation for another user to join a Space or Project
    Create,
    /// List sharing invitations you've created or received
    List(ListCommand),
    /// Revoke a sharing invitation you've previously created
    Revoke,
    /// Create a sharing invitation for a single service
    Service(ServiceCreateCommand),
}

impl ShareCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        use ShareSubcommand::*;
        match self.subcommand {
            Accept => todo!(),
            Create => todo!(),
            List(c) => c.run(options),
            Revoke => todo!(),
            Service(c) => c.run(options),
        }
    }
}
