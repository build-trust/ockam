use clap::{Args, Subcommand};

use crate::CommandGlobalOpts;

mod accept;
mod create;
mod list;
mod output;
mod service;
mod show;

pub use accept::AcceptCommand;
pub use create::CreateCommand;
pub use list::ListCommand;
pub use service::ServiceCreateCommand;
pub use show::ShowCommand;

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
    Accept(AcceptCommand),
    /// Create an invitation for another user to join a Space or Project
    Create(CreateCommand),
    /// List sharing invitations you've created or received
    List(ListCommand),
    /// Revoke a sharing invitation you've previously created
    Revoke,
    /// Create a sharing invitation for a single service
    Service(ServiceCreateCommand),
    /// Show information about a single invitation you own or received, including service access details
    Show(ShowCommand),
}

impl ShareCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        use ShareSubcommand::*;
        match self.subcommand {
            Accept(c) => c.run(options),
            Create(c) => c.run(options),
            List(c) => c.run(options),
            Revoke => todo!(),
            Service(c) => c.run(options),
            Show(c) => c.run(options),
        }
    }
}
