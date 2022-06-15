use clap::{Args, Subcommand};

use accept::AcceptCommand;
use create::CreateCommand;
use list::ListCommand;
use ockam_multiaddr::MultiAddr;
use reject::RejectCommand;

use crate::HELP_TEMPLATE;

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
        default_value = "/dnsaddr/cloud.ockam.io/tcp/62526"
    )]
    pub cloud_addr: MultiAddr,
}

#[derive(Clone, Debug, Subcommand)]
pub enum InvitationSubcommand {
    /// Create invitations
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    // list invitations
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Accept(AcceptCommand),

    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Reject(RejectCommand),
}

impl InvitationCommand {
    pub fn run(inv_cmd: InvitationCommand) {
        match inv_cmd.subcommand {
            InvitationSubcommand::Create(command) => {
                CreateCommand::run(command, inv_cmd.cloud_addr)
            }
            InvitationSubcommand::List(command) => ListCommand::run(command, inv_cmd.cloud_addr),
            InvitationSubcommand::Accept(command) => {
                AcceptCommand::run(command, inv_cmd.cloud_addr)
            }
            InvitationSubcommand::Reject(command) => {
                RejectCommand::run(command, inv_cmd.cloud_addr)
            }
        }
    }
}
