use clap::{Args, Subcommand};

use crate::CommandGlobalOpts;

mod list;
mod output;

pub use list::ListCommand;

/// Manage sharing invitations in Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct ShareCommand {
    #[command(subcommand)]
    subcommand: ShareSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ShareSubcommand {
    /// Accept a received sharing invitation
    Accept,
    /// List sharing invitations you've created or received
    List(ListCommand),
    /// Revoke a sharing invitation you've previously created
    Revoke,
    /// Create a sharing invitation for a single service
    Service,
}

impl ShareCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        use ShareSubcommand::*;
        match self.subcommand {
            Accept => todo!(),
            List(c) => c.run(options),
            Revoke => todo!(),
            Service => todo!(),
        }
    }
}
