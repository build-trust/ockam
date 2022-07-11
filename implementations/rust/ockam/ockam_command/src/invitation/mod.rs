use clap::{Args, Subcommand};

pub use accept::AcceptCommand;
pub use create::CreateCommand;
pub use list::ListCommand;
use ockam_multiaddr::MultiAddr;
pub use reject::RejectCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};

mod accept;
mod create;
mod list;
mod reject;

#[derive(Clone, Debug, Args)]
pub struct InvitationCommand {
    #[clap(subcommand)]
    subcommand: InvitationSubcommand,

    /// Ockam's cloud node address
    #[clap(
        global = true,
        display_order = 1000,
        long,
        default_value = "/dnsaddr/cloud.ockam.io/tcp/62526/ockam/api"
    )]
    pub cloud_addr: MultiAddr,
}

#[derive(Clone, Debug, Subcommand)]
pub enum InvitationSubcommand {
    /// Create invitations
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// List pending invitations
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Accept an invitation
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Accept(AcceptCommand),

    /// Reject an invitation
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Reject(RejectCommand),
}

impl InvitationCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: InvitationCommand) {
        match cmd.subcommand {
            InvitationSubcommand::Create(command) => CreateCommand::run(opts, command),
            InvitationSubcommand::List(command) => ListCommand::run(opts, command),
            InvitationSubcommand::Accept(command) => AcceptCommand::run(opts, command),
            InvitationSubcommand::Reject(command) => RejectCommand::run(opts, command),
        }
    }
}
