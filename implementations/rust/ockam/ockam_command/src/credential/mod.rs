use clap::{Args, Subcommand};
use colorful::Colorful;

pub(crate) use get::GetCommand;
pub(crate) use issue::IssueCommand;
pub(crate) use list::ListCommand;
use ockam_api::cli_state::NamedCredential;
pub(crate) use present::PresentCommand;
pub(crate) use show::ShowCommand;
pub(crate) use store::StoreCommand;
pub(crate) use verify::VerifyCommand;

use crate::output::{CredentialAndPurposeKeyDisplay, Output};
use crate::{CommandGlobalOpts, Result};

pub(crate) mod get;
pub(crate) mod issue;
pub(crate) mod list;
pub(crate) mod present;
pub(crate) mod show;
pub(crate) mod store;
pub(crate) mod verify;

/// Manage Credentials
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
pub struct CredentialCommand {
    #[command(subcommand)]
    subcommand: CredentialSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CredentialSubcommand {
    #[command(display_order = 900)]
    Get(GetCommand),
    Issue(IssueCommand),
    List(ListCommand),
    Present(PresentCommand),
    Show(ShowCommand),
    Store(StoreCommand),
    Verify(VerifyCommand),
}

impl CredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        match self.subcommand {
            CredentialSubcommand::Get(c) => c.run(options),
            CredentialSubcommand::Issue(c) => c.run(options),
            CredentialSubcommand::List(c) => c.run(options),
            CredentialSubcommand::Present(c) => c.run(options),
            CredentialSubcommand::Show(c) => c.run(options),
            CredentialSubcommand::Store(c) => c.run(options),
            CredentialSubcommand::Verify(c) => c.run(options),
        }
    }
}

pub struct CredentialOutput {
    name: String,
    credential: String,
    is_verified: bool,
}

impl CredentialOutput {
    pub async fn new(credential: NamedCredential) -> Self {
        Self {
            name: credential.name(),
            credential: format!(
                "{}",
                CredentialAndPurposeKeyDisplay(credential.credential_and_purpose_key())
            ),
            is_verified: true,
        }
    }
}

impl Output for CredentialOutput {
    fn output(&self) -> Result<String> {
        let is_verified = if self.is_verified {
            "✔︎".light_green()
        } else {
            "✕".light_red()
        };
        let output = format!(
            "Credential: {cred_name} {is_verified}\n{cred}",
            cred_name = self.name,
            is_verified = is_verified,
            cred = self.credential
        );

        Ok(output)
    }
}
