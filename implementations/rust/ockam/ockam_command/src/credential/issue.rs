use std::collections::HashMap;

use crate::{
    identity::default_identity_name,
    util::{node_rpc, print_encodable},
    vault::default_vault_name,
    CommandGlobalOpts, EncodeFormat, Result,
};
use anyhow::Context as _;
use clap::Args;
use ockam::Context;
use ockam_identity::{credential::Credential, IdentityIdentifier};

#[derive(Clone, Debug, Args)]
pub struct IssueCommand {
    #[arg(long = "as", default_value_t = default_identity_name())]
    pub as_identity: String,

    #[arg(long = "for", value_name = "IDENTITY_ID")]
    pub for_identity: IdentityIdentifier,

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
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, IssueCommand),
) -> crate::Result<()> {
    let attrs = cmd.attributes()?;
    let cred_builder = attrs.iter().fold(
        Credential::builder(cmd.for_identity.clone()),
        |crd, (k, v)| crd.with_attribute(k, v.as_bytes()),
    );

    let vault = opts.state.vaults.get(&cmd.vault)?.get().await?;
    let ident_state = opts.state.identities.get(&cmd.as_identity)?;

    let ident = ident_state.get(&ctx, &vault).await?;

    let credential = ident.issue_credential(cred_builder).await?;

    print_encodable(credential, &cmd.encode_format)?;

    Ok(())
}
