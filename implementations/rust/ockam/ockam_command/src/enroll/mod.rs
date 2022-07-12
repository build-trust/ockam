use clap::Args;

pub use auth0::Auth0Service;
use auth0::*;
use enrollment_token_authenticate::*;
pub use enrollment_token_generate::GenerateEnrollmentTokenCommand;

use crate::enroll::email::EnrollEmailCommand;
use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::CommandGlobalOpts;

mod auth0;
mod email;
mod enrollment_token_authenticate;
mod enrollment_token_generate;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    /// Authenticates an enrollment token
    #[clap(display_order = 1003, long, group = "enroll_params")]
    pub token: Option<String>,

    /// Enroll using the Auth0 flow
    #[clap(display_order = 1004, long, group = "enroll_params")]
    auth0: bool,

    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
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
