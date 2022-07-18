use clap::{Args, Subcommand};

pub use accept::AcceptCommand;
pub use create::CreateCommand;
pub use list::ListCommand;
pub use reject::RejectCommand;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::{CommandGlobalOpts, HELP_TEMPLATE};

mod accept;
mod create;
mod list;
mod reject;

#[derive(Clone, Debug, Args)]
pub struct InvitationCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(flatten)]
    cloud_opts: CloudOpts,

    #[clap(subcommand)]
    subcommand: InvitationSubcommand,
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
            InvitationSubcommand::Create(scmd) => {
                CreateCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
            InvitationSubcommand::List(scmd) => {
                ListCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
            InvitationSubcommand::Accept(scmd) => {
                AcceptCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
            InvitationSubcommand::Reject(scmd) => {
                RejectCommand::run(opts, (cmd.cloud_opts, cmd.node_opts), scmd)
            }
        }
    }
}
