pub(crate) mod get;
pub(crate) mod issue;
pub(crate) mod list;
pub(crate) mod present;
pub(crate) mod show;
pub(crate) mod store;
pub(crate) mod verify;

use anyhow::anyhow;
pub(crate) use get::GetCommand;
pub(crate) use issue::IssueCommand;
pub(crate) use list::ListCommand;
use ockam::identity::credential::{Credential, CredentialData, Unverified};
use ockam::identity::IdentityIdentifier;
pub(crate) use present::PresentCommand;
pub(crate) use show::ShowCommand;
pub(crate) use store::StoreCommand;
pub(crate) use verify::VerifyCommand;

use crate::{docs, CommandGlobalOpts, Result};
use clap::{Args, Subcommand};
use ockam_api::cli_state::traits::StateDirTrait;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(
    after_long_help = docs::after_help(HELP_DETAIL),
    arg_required_else_help = true,
    subcommand_required = true
)]
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

pub async fn validate_encoded_cred(
    encoded_cred: &str,
    issuer: &IdentityIdentifier,
    vault: &str,
    opts: &CommandGlobalOpts,
) -> Result<()> {
    let vault = opts.state.vaults.get(vault)?.get().await?;
    let identities = opts.state.get_identities(vault).await?;
    let identity = identities.repository().get_identity(&issuer).await?;

    let bytes = match hex::decode(encoded_cred) {
        Ok(b) => b,
        Err(e) => return Err(anyhow!(e).into()),
    };
    let cred: Credential = minicbor::decode(&bytes)?;
    let cred_data: CredentialData<Unverified> = minicbor::decode(cred.unverified_data())?;

    identities
        .credentials()
        .verify_credential(cred_data.unverified_subject(), &[identity], cred)
        .await?;

    Ok(())
}
