use clap::Args;
use miette::{IntoDiagnostic, miette};

use ockam::Context;
use ockam::identity::CredentialData;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_core::compat::collections::HashMap;
use ockam_identity::{identities, Identity};

use crate::{
    CommandGlobalOpts,
    docs,
    EncodeFormat,
    Result, util::{node_rpc, print_encodable}, vault::default_vault_name,
};
use crate::identity::{get_identity_name, initialize_identity_if_default};

const LONG_ABOUT: &str = include_str!("./static/issue/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/issue/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct IssueCommand {
    #[arg(long = "as")]
    pub as_identity: Option<String>,

    #[arg(long = "for", value_name = "IDENTITY_ID")]
    pub for_identity: String,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    pub attributes: Vec<String>,

    #[arg()]
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

    pub async fn identity(&self) -> Result<Identity> {
        let identity_as_bytes = match hex::decode(&self.for_identity) {
            Ok(b) => b,
            Err(e) => return Err(miette!(e).into()),
        };

        let identity = identities()
            .identities_creation()
            .decode_identity(&identity_as_bytes)
            .await?;
        Ok(identity)
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, IssueCommand),
) -> miette::Result<()> {
    let identity_name = get_identity_name(&opts.state, &cmd.as_identity);
    let ident_state = opts.state.identities.get(&identity_name)?;
    let auth_identity_identifier = ident_state.config().identifier().clone();

    let mut attrs = cmd.attributes()?;
    attrs.insert(
        "project_id".to_string(), // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
        auth_identity_identifier.to_string(),
    );
    attrs.insert(
        "trust_context_id".to_string(),
        auth_identity_identifier.to_string(),
    );

    let vault_name = cmd
        .vault
        .clone()
        .unwrap_or_else(|| default_vault_name(&opts.state));
    let vault = opts.state.vaults.get(&vault_name)?.get().await?;
    let identities = opts.state.get_identities(vault).await?;
    let issuer = ident_state.identifier();

    let credential_data =
        CredentialData::from_attributes(cmd.identity().await?.identifier(), issuer.clone(), attrs)
            .into_diagnostic()?;
    let credential = identities
        .credentials()
        .issue_credential(&issuer, credential_data)
        .await
        .into_diagnostic()?;

    print_encodable(credential, &cmd.encode_format)?;

    Ok(())
}
