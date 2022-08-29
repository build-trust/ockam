pub(crate) mod get_credential;
pub(crate) mod present_credential;
pub(crate) mod set_authority;

pub(crate) use get_credential::GetCredentialCommand;
pub(crate) use present_credential::PresentCredentialCommand;
pub(crate) use set_authority::SetAuthorityCommand;

use crate::help;
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
#[clap(
    hide = help::hide(),
    help_template = help::template(""),
    arg_required_else_help = true,
    subcommand_required = true
)]
pub struct CredentialCommand {
    #[clap(subcommand)]
    subcommand: CredentialSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CredentialSubcommand {
    SetAuthority(SetAuthorityCommand),
    Get(GetCredentialCommand),
    Present(PresentCredentialCommand),
}

impl CredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            CredentialSubcommand::SetAuthority(c) => c.run(options),
            CredentialSubcommand::Get(c) => c.run(options),
            CredentialSubcommand::Present(c) => c.run(options),
        }
    }
}
