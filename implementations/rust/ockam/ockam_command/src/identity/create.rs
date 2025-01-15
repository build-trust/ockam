use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam::identity::models::ChangeHistory;
use ockam::identity::IdentitiesVerification;
use ockam_api::cli_state::journeys::{JourneyEvent, IDENTIFIER, IDENTITY_NAME};
use ockam_api::cli_state::{random_name, NamedVault};
use ockam_api::colors::{color_primary, OckamColor};
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use ockam_vault::SoftwareVaultForVerifyingSignatures;
use std::collections::HashMap;

use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[arg(hide_default_value = true, default_value_t = random_name())]
    pub name: String,

    /// The name of the Vault where the Identity key will be stored
    #[arg(long, value_name = "VAULT_NAME")]
    pub vault: Option<String>,

    /// Key ID to use for the identity creation
    #[arg(short, long)]
    pub key_id: Option<String>,

    /// Identity to import in hex format
    #[arg(long, value_name = "IDENTITY", conflicts_with = "key_id")]
    identity: Option<String>,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "identity create";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let _notification_handler = NotificationHandler::start(&opts.state, opts.terminal.clone());
        let vault = match &self.vault {
            Some(vault_name) => opts.state.get_or_create_named_vault(vault_name).await?,
            None => opts.state.get_or_create_default_named_vault().await?,
        };
        if let Some(identity) = self.identity.clone() {
            self.import(opts, vault, identity).await?;
        } else {
            self.create(opts, vault).await?;
        };
        Ok(())
    }
}

impl CreateCommand {
    async fn create(self, opts: CommandGlobalOpts, vault: NamedVault) -> miette::Result<()> {
        let identity = match &self.key_id {
            Some(key_id) => {
                opts.state
                    .create_identity_with_key_id(&self.name, &vault.name(), key_id.as_ref())
                    .await?
            }
            None => {
                opts.state
                    .create_identity_with_name_and_vault(&self.name, &vault.name())
                    .await?
            }
        };
        let identifier = identity.identifier().to_string();
        let mut attributes = HashMap::new();
        attributes.insert(IDENTIFIER, identifier.clone());
        attributes.insert(IDENTITY_NAME, self.name.clone());
        opts.state
            .add_journey_event(JourneyEvent::IdentityCreated, attributes)
            .await?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!(
                    "Identity {} \n",
                    identifier
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ) + &fmt_log!(
                    "created successfully as {}",
                    &self
                        .name
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ),
            )
            .machine(identifier.clone())
            .json(serde_json::json!({ "identifier": &identifier }))
            .write_line()?;

        Ok(())
    }

    async fn import(
        self,
        opts: CommandGlobalOpts,
        vault: NamedVault,
        identity: String,
    ) -> miette::Result<()> {
        let change_history = ChangeHistory::import_from_string(&identity).into_diagnostic()?;
        let identifier = IdentitiesVerification::new(
            opts.state.change_history_repository(),
            SoftwareVaultForVerifyingSignatures::create(),
        )
        .import_from_change_history(None, change_history)
        .await
        .into_diagnostic()?;
        opts.state
            .store_named_identity(&identifier, &self.name, &vault.name())
            .await?;

        let mut attributes = HashMap::new();
        attributes.insert(IDENTIFIER, identifier.to_string());
        attributes.insert(IDENTITY_NAME, self.name.clone());
        opts.state
            .add_journey_event(JourneyEvent::IdentityCreated, attributes)
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Identity imported successfully with name {}",
                color_primary(&self.name)
            ))
            .write_line()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }
}
