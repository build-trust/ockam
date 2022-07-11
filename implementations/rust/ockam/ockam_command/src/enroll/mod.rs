use clap::Args;

pub use auth0::Auth0Service;
use auth0::*;
use enrollment_token_authenticate::*;
pub use enrollment_token_generate::GenerateEnrollmentTokenCommand;
use ockam_multiaddr::MultiAddr;

use crate::enroll::email::EnrollEmailCommand;
use crate::node::NodeOpts;
use crate::CommandGlobalOpts;

mod auth0;
mod email;
mod enrollment_token_authenticate;
mod enrollment_token_generate;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    /// Ockam's cloud secure channel address
    #[clap(
        display_order = 1000,
        default_value = "/dnsaddr/ockam.cloud.io/tcp/4000"
    )]
    address: MultiAddr,

    /// Authenticates an enrollment token
    #[clap(display_order = 1003, long, group = "enroll_params")]
    pub token: Option<String>,

    /// Enroll using the Auth0 flow
    #[clap(display_order = 1004, long, group = "enroll_params")]
    auth0: bool,

    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl EnrollCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: EnrollCommand) {
        if cmd.token.is_some() {
            AuthenticateEnrollmentTokenCommand::run(opts, cmd)
        } else if cmd.auth0 {
            EnrollAuth0Command::run(opts, cmd)
        } else {
            EnrollEmailCommand::run(opts, cmd)
        }
    }
}
