use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use miette::IntoDiagnostic;
use ockam::identity::Identity;
use ockam_api::orchestrator::email_address::EmailAddress;
use ockam_node::Context;
use ockam_vault::SigningSecret;
use serde::{Deserialize, Serialize};

const LONG_ABOUT: &str = include_str!("./static/export/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/export/after_long_help.txt");

/// Export an identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ExportCommand {
    name: Option<String>,
}

#[async_trait]
impl Command for ExportCommand {
    const NAME: &'static str = "identity export";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let identity_name = match &self.name {
            Some(name) => name.clone(),
            None => opts.state.get_default_identity_name().await?,
        };
        let enrolled_email = opts
            .state
            .get_identity_enrollment(&identity_name)
            .await?
            .and_then(|i| i.status().email().cloned());
        let named_identity = opts.state.get_named_identity(&identity_name).await?;
        let named_vault = opts
            .state
            .get_named_vault(&named_identity.vault_name())
            .await?;
        let vault = opts.state.make_vault(named_vault).await?;
        let identities = opts.state.make_identities(vault).await?;
        let vault = identities.vault();
        let identity = opts
            .state
            .get_identity_by_optional_name(Some(&identity_name))
            .await?;
        let signing_secret_key_handle = identities
            .identities_keys()
            .get_secret_key(&identity)
            .await?;
        let signing_secret_key = vault
            .identity_vault
            .export_key(&signing_secret_key_handle)
            .await?;
        let exported_identity =
            ExportedIdentity::new(&identity_name, enrolled_email, identity, signing_secret_key)?;
        let as_string = exported_identity.export()?;
        opts.terminal.to_stdout().machine(as_string).write_line()?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct ExportedIdentity {
    pub name: String,
    pub enrolled_email: Option<String>,
    pub change_history: String,
    signing_secret: String,
    signing_secret_type: u8,
}

impl ExportedIdentity {
    pub fn new(
        name: &str,
        enrolled_email: Option<EmailAddress>,
        identity: Identity,
        signing_secret: SigningSecret,
    ) -> miette::Result<Self> {
        Ok(ExportedIdentity {
            name: name.to_string(),
            enrolled_email: enrolled_email.map(|e| e.to_string()),
            change_history: identity.export_as_string()?,
            signing_secret: hex::encode(signing_secret.key()),
            signing_secret_type: signing_secret.type_as_u8(),
        })
    }

    pub fn from_hex(hex: &str) -> miette::Result<Self> {
        let as_json = hex::decode(hex).into_diagnostic()?;
        serde_json::from_slice(&as_json).into_diagnostic()
    }

    pub fn export(&self) -> miette::Result<String> {
        let as_json = serde_json::to_string(&self).into_diagnostic()?;
        let as_hex = hex::encode(as_json);
        Ok(as_hex)
    }

    pub fn hex_decoded_change_history(&self) -> miette::Result<Vec<u8>> {
        hex::decode(&self.change_history).into_diagnostic()
    }

    pub fn signing_secret(&self) -> miette::Result<SigningSecret> {
        let key = hex::decode(&self.signing_secret).into_diagnostic()?;
        let key_type = self.signing_secret_type;
        SigningSecret::from_key(&key, key_type).into_diagnostic()
    }
}
