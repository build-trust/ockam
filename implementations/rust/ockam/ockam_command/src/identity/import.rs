use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::random_name;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_node::Context;

use crate::identity::export::ExportedIdentity;
use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/import/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/import/after_long_help.txt");

/// Import an identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ImportCommand {
    exported: String,
}

#[async_trait]
impl Command for ImportCommand {
    const NAME: &'static str = "identity import";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let exported_identity = ExportedIdentity::from_hex(&self.exported)?;
        let signing_secret = exported_identity.signing_secret()?;
        let change_history = exported_identity.hex_decoded_change_history()?;
        let user_email = exported_identity.user_email()?;
        let user = exported_identity.user()?;
        let identity_name = if opts
            .state
            .get_named_identity(&exported_identity.name)
            .await
            .is_ok()
        {
            random_name()
        } else {
            exported_identity.name
        };

        let named_vault = opts.state.get_or_create_default_named_vault().await?;
        let vault_name = named_vault.name();
        let vault = opts.state.make_vault(named_vault).await?;
        let identities = opts.state.make_identities(vault).await?;
        let vault = identities.vault();

        let signing_secret_key_handle = vault.identity_vault.import_key(signing_secret).await?;
        let identifier = identities
            .identities_creation()
            .import_private_identity(None, change_history.as_slice(), &signing_secret_key_handle)
            .await?;

        let named_identity = opts
            .state
            .store_named_identity(&identifier, &identity_name, &vault_name)
            .await?;

        if let Some(user_email) = user_email.as_ref() {
            opts.state
                .set_identifier_as_enrolled(&identifier, user_email)
                .await?;
        }
        if let Some(user) = user {
            opts.state
                .set_identifier_as_enrolled(&identifier, &user.email)
                .await?;
            opts.state.store_user(&user).await?;
        }

        opts.terminal
            .to_stdout()
            .plain(fmt_ok!(
                "Identity {} imported successfully",
                color_primary(named_identity.name())
            ))
            .machine(named_identity.name())
            .write_line()?;
        Ok(())
    }
}
