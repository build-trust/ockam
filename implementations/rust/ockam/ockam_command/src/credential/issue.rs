use ockam_core::compat::collections::HashMap;

use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::{
    util::{node_rpc, parsers::identity_identifier_parser},
    vault::default_vault_name,
    CommandGlobalOpts, Result,
};
use clap::Args;

use crate::output::{CredentialAndPurposeKeyDisplay, EncodeFormat};
use miette::{miette, IntoDiagnostic};
use ockam::identity::utils::AttributesBuilder;
use ockam::identity::Identifier;
use ockam::identity::{MAX_CREDENTIAL_VALIDITY, PROJECT_MEMBER_SCHEMA, TRUST_CONTEXT_ID};
use ockam::Context;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};

#[derive(Clone, Debug, Args)]
pub struct IssueCommand {
    /// Name of the Identity to be used as the credential issuer
    #[arg(long = "as", value_name = "IDENTITY_NAME")]
    pub as_identity: Option<String>,

    #[arg(long = "for", value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
    pub identity_identifier: Identifier,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    pub attributes: Vec<String>,

    /// Name of the Vault that will be used to issue the credential.
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,

    /// Encoding Format
    #[arg(long = "encoding", value_enum, default_value = "plain")]
    encode_format: EncodeFormat,
}

impl IssueCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.as_identity);
        node_rpc(run_impl, (opts, self));
    }

    fn attributes(&self) -> Result<HashMap<String, String>> {
        let mut attributes = HashMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().ok_or(miette!("key expected"))?;
            let value = parts.next().ok_or(miette!("value expected)"))?;
            attributes.insert(key.to_string(), value.to_string());
        }
        Ok(attributes)
    }

    pub fn identity_identifier(&self) -> &Identifier {
        &self.identity_identifier
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, IssueCommand),
) -> miette::Result<()> {
    let identity_name = get_identity_name(&opts.state, &cmd.as_identity);
    let ident_state = opts.state.identities.get(&identity_name)?;
    let auth_identity_identifier = ident_state.config().identifier().clone();

    let vault_name = cmd
        .vault
        .clone()
        .unwrap_or_else(|| default_vault_name(&opts.state));
    let vault = opts.state.vaults.get(&vault_name)?.get().await?;
    let identities = opts.state.get_identities(vault).await?;
    let issuer = ident_state.identifier();

    let mut attributes_builder = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA)
        .with_attribute(
            TRUST_CONTEXT_ID.to_vec(),
            auth_identity_identifier.to_string(),
        );
    for (key, value) in cmd.attributes()? {
        attributes_builder =
            attributes_builder.with_attribute(key.as_bytes().to_vec(), value.as_bytes().to_vec());
    }

    let credential = identities
        .credentials()
        .credentials_creation()
        .issue_credential(
            &issuer,
            cmd.identity_identifier(),
            attributes_builder.build(),
            MAX_CREDENTIAL_VALIDITY,
        )
        .await
        .into_diagnostic()?;

    cmd.encode_format
        .println_value(&CredentialAndPurposeKeyDisplay(credential))?;

    Ok(())
}
