use clap::{Args, Subcommand};

use auth0::*;
use enrollment_token_authenticate::*;
pub use enrollment_token_generate::GenerateEnrollmentTokenCommand;

use crate::HELP_TEMPLATE;

mod auth0;
mod enrollment_token_authenticate;
mod enrollment_token_generate;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    #[clap(subcommand)]
    subcommand: EnrollSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum EnrollSubcommand {
    /// Authenticate using the Auth0 flow
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Auth0(EnrollAuth0Command),

    /// Authenticates an enrollment token
    #[clap(display_order = 900, help_template = HELP_TEMPLATE, name = "token")]
    AuthenticateEnrollmentToken(AuthenticateEnrollmentTokenCommand),
}

impl EnrollCommand {
    pub fn run(command: EnrollCommand) {
        match command.subcommand {
            EnrollSubcommand::Auth0(command) => EnrollAuth0Command::run(command),
            EnrollSubcommand::AuthenticateEnrollmentToken(command) => {
                AuthenticateEnrollmentTokenCommand::run(command)
            }
        }
    }
}
