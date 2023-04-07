use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_vault::Vault;

use crate::{
    identity::default_identity_name,
    util::{node_rpc, print_encodable},
    vault::default_vault_name,
    CommandGlobalOpts, EncodeFormat, Result,
};
use anyhow::{anyhow, Context as _};
use clap::Args;
use ockam::Context;
use ockam_identity::{credential::CredentialBuilder, PublicIdentity};

#[derive(Clone, Debug, Args)]
pub struct IssueCommand {
    #[arg(long = "as", default_value_t = default_identity_name())]
    pub as_identity: String,

    #[arg(long = "for", value_name = "IDENTITY_ID")]
    pub for_identity: String,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    pub attributes: Vec<String>,

    #[arg(default_value_t = default_vault_name())]
    pub vault: String,

    /// Encoding Format
    #[arg(long = "encoding", value_enum, default_value = "plain")]
    encode_format: EncodeFormat,
}

impl IssueCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }

    fn attributes(&self) -> Result<HashMap<String, String>> {
        let mut attributes = HashMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().context("key expected")?;
            let value = parts.next().context("value expected)")?;
            attributes.insert(key.to_string(), value.to_string());
        }
        Ok(attributes)
    }

    pub async fn public_identity(&self) -> Result<PublicIdentity> {
        let identity_as_bytes = match hex::decode(&self.for_identity) {
            Ok(b) => b,
            Err(e) => return Err(anyhow!(e).into()),
        };

        let public_identity = PublicIdentity::import(&identity_as_bytes, Vault::create()).await?;
        Ok(public_identity)
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, IssueCommand),
) -> crate::Result<()> {
    let ident_state = opts.state.identities.get(&cmd.as_identity)?;
    let auth_identity_identifier = ident_state.config.identifier.clone();

    let mut attrs = cmd.attributes()?;
    attrs.insert(
        "project_id".to_string(), // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
        auth_identity_identifier.to_string(),
    );
    attrs.insert(
        "trust_context_id".to_string(),
        auth_identity_identifier.to_string(),
    );

    let cred_builder = CredentialBuilder::from_attributes(
        cmd.public_identity().await?.identifier().clone(),
        attrs,
    );

    let vault = opts.state.vaults.get(&cmd.vault)?.get().await?;

    let ident = ident_state.get(&ctx, Arc::new(vault)).await?;

    let credential = ident.issue_credential(cred_builder).await?;

    print_encodable(credential, &cmd.encode_format)?;

    Ok(())
}
