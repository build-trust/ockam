use clap::Args;

use auth0::*;
use enrollment_token_authenticate::*;
pub use enrollment_token_generate::GenerateEnrollmentTokenCommand;
use ockam_multiaddr::MultiAddr;

use crate::enroll::email::EnrollEmailCommand;
use crate::IdentityOpts;

mod auth0;
mod email;
mod enrollment_token_authenticate;
mod enrollment_token_generate;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    /// Ockam's cloud address
    #[clap(
        display_order = 1000,
        default_value = "/dnsaddr/ockam.cloud.io/tcp/4000"
    )]
    address: MultiAddr,

    #[clap(display_order = 1001, long, default_value = "default")]
    vault: String,

    #[clap(display_order = 1002, long, default_value = "default")]
    identity: String,

    /// Authenticates an enrollment token
    #[clap(display_order = 1003, long, group = "enroll_params")]
    token: Option<String>,

    /// Enroll using the Auth0 flow
    #[clap(display_order = 1004, long, group = "enroll_params")]
    auth0: bool,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl EnrollCommand {
    pub fn run(command: EnrollCommand) {
        if command.token.is_some() {
            AuthenticateEnrollmentTokenCommand::run(command)
        } else if command.auth0 {
            EnrollAuth0Command::run(command)
        } else {
            EnrollEmailCommand::run(command)
        }
    }
}
