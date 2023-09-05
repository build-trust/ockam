pub(crate) mod get;
pub(crate) mod issue;
pub(crate) mod list;
pub(crate) mod present;
pub(crate) mod show;
pub(crate) mod store;
pub(crate) mod verify;

use colorful::Colorful;
pub(crate) use get::GetCommand;
pub(crate) use issue::IssueCommand;
pub(crate) use list::ListCommand;
use ockam::identity::{Identifier, Identities, Identity};
use ockam_api::cli_state::{CredentialState, StateItemTrait};
pub(crate) use present::PresentCommand;
pub(crate) use show::ShowCommand;
use std::sync::Arc;
pub(crate) use store::StoreCommand;
pub(crate) use verify::VerifyCommand;

use crate::output::{CredentialAndPurposeKeyDisplay, Output};
use crate::{CommandGlobalOpts, Result};
use clap::{Args, Subcommand};
use miette::IntoDiagnostic;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam_api::cli_state::traits::StateDirTrait;

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

pub async fn identities(vault_name: &str, opts: &CommandGlobalOpts) -> Result<Arc<Identities>> {
    let vault = opts.state.vaults.get(vault_name)?.get().await?;
    let identities = opts.state.get_identities(vault).await?;

    Ok(identities)
}

pub async fn identity(identity: &str, identities: Arc<Identities>) -> Result<Identity> {
    let identity_as_bytes = hex::decode(identity)?;

    let identity = identities
        .identities_creation()
        .import(None, &identity_as_bytes)
        .await?;

    Ok(identity)
}

pub async fn validate_encoded_cred(
    encoded_cred: &[u8],
    identities: Arc<Identities>,
    issuer: &Identifier,
) -> Result<()> {
    let cred: CredentialAndPurposeKey = minicbor::decode(encoded_cred)?;

    identities
        .credentials()
        .credentials_verification()
        .verify_credential(None, &[issuer.clone()], &cred)
        .await?;

    Ok(())
}

pub struct CredentialOutput {
    name: String,
    credential: String,
    is_verified: bool,
}

impl CredentialOutput {
    pub async fn try_from_state(
        opts: &CommandGlobalOpts,
        state: &CredentialState,
        vault_name: &str,
    ) -> Result<Self> {
        let config = state.config();

        let identities = identities(vault_name, opts).await.into_diagnostic()?;

        let is_verified = validate_encoded_cred(
            &config.encoded_credential,
            identities,
            &config.issuer_identifier,
        )
        .await
        .is_ok();

        let credential = config.credential()?;
        let credential = format!("{}", CredentialAndPurposeKeyDisplay(credential));

        let output = Self {
            name: state.name().to_string(),
            credential,
            is_verified,
        };

        Ok(output)
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
