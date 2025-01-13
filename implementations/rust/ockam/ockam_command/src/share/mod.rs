use clap::{Args, Subcommand};

pub use accept::AcceptCommand;
pub use create::CreateCommand;
pub use list::ListCommand;
pub use service::ServiceCreateCommand;
pub use show::ShowCommand;

use crate::{docs, CommandGlobalOpts};

mod accept;
mod create;
mod list;
mod service;
mod show;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true,
about=docs::about("Manage sharing invitations in Ockam Orchestrator"))]
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        use ShareSubcommand::*;
        match self.subcommand {
            Accept(c) => c.run(opts),
            Create(c) => c.run(opts),
            List(c) => c.run(opts),
            Revoke => todo!(),
            Service(c) => c.run(opts),
            Show(c) => c.run(opts),
        }
    }

    pub fn name(&self) -> String {
        match &self.subcommand {
            ShareSubcommand::Accept(c) => c.name(),
            ShareSubcommand::Create(c) => c.name(),
            ShareSubcommand::List(c) => c.name(),
            ShareSubcommand::Show(c) => c.name(),
            ShareSubcommand::Service(c) => c.name(),
            ShareSubcommand::Revoke => "revoke invitation".to_string(),
        }
    }
}
