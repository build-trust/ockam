use clap::Args;
use miette::{miette, IntoDiagnostic};

use ockam::identity::utils::AttributesBuilder;
use ockam::identity::Identifier;
use ockam_api::authenticator::credential_issuer::PROJECT_MEMBER_SCHEMA;
use ockam_api::output::{EncodeFormat, Output};
use ockam_core::compat::collections::HashMap;

use crate::credential::CredentialOutput;
use crate::output::CredentialAndPurposeKeyDisplay;
use crate::util::parsers::duration_parser;
use crate::{util::parsers::identity_identifier_parser, CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct IssueCommand {
    /// Name of the Identity to be used as the credential issuer
    #[arg(long = "as", value_name = "IDENTITY_NAME")]
    pub as_identity: Option<String>,

    /// Identifier of the Identity that the credential is issued for
    #[arg(long = "for", value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
    pub identity_identifier: Identifier,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    pub attributes: Vec<String>,

    /// The name of the Vault that will be used to issue the credential.
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,

    /// Encoding Format
    #[arg(long = "encoding", value_enum, default_value = "plain")]
    encode_format: EncodeFormat,

    /// Time to live for the credential
    #[arg(long, value_name = "TTL", default_value = "30m", value_parser = duration_parser)]
    ttl: std::time::Duration,
}

impl IssueCommand {
    pub fn name(&self) -> String {
        "credential issue".into()
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

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let authority = opts
            .state
            .get_identifier_by_optional_name(&self.as_identity)
            .await?;

        let named_vault = opts.state.get_named_vault_or_default(&self.vault).await?;
        let vault = opts.state.make_vault(named_vault).await?;
        let identities = opts.state.make_identities(vault).await?;

        let mut attributes_builder = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA);
        for (key, value) in self.attributes()? {
            attributes_builder = attributes_builder
                .with_attribute(key.as_bytes().to_vec(), value.as_bytes().to_vec());
        }

        let credential = identities
            .credentials()
            .credentials_creation()
            .issue_credential(
                &authority,
                &self.identity_identifier,
                attributes_builder.build(),
                self.ttl,
            )
            .await
            .into_diagnostic()?;

        let machine = self
            .encode_format
            .encode_value(&CredentialAndPurposeKeyDisplay(credential.clone()))?;
        let output = CredentialOutput::from_credential(credential)?;
        opts.terminal
            .stdout()
            .plain(output.item()?)
            .json_obj(output)?
            .machine(machine)
            .write_line()?;

        Ok(())
    }
}
