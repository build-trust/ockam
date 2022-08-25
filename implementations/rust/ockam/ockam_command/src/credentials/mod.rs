pub(crate) mod get_credential;
pub(crate) mod present_credential;
pub(crate) mod set_authority;

pub(crate) use get_credential::GetCredentialCommand;
pub(crate) use present_credential::PresentCredentialCommand;
pub(crate) use set_authority::SetAuthorityCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct CredentialsCommand {
    #[clap(subcommand)]
    subcommand: CredentialsSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CredentialsSubcommand {
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    SetAuthority(SetAuthorityCommand),

    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Get(GetCredentialCommand),

    /// An authorised member can request a credential from the projects's authority.
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Present(PresentCredentialCommand),
}

impl CredentialsCommand {
    pub fn run(opts: CommandGlobalOpts, command: CredentialsCommand) {
        match command.subcommand {
            CredentialsSubcommand::SetAuthority(command) => SetAuthorityCommand::run(opts, command),
            CredentialsSubcommand::Get(command) => GetCredentialCommand::run(opts, command),
            CredentialsSubcommand::Present(command) => PresentCredentialCommand::run(opts, command),
        }
    }
}
