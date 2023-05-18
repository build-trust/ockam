use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_identity::IdentityIdentifier;
use rand::prelude::random;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[arg(hide_default_value = true, default_value_t = hex::encode(& random::< [u8; 4] > ()))]
    name: String,

    /// Vault name to store the identity key
    #[arg(long)]
    vault: Option<String>,
}

impl CreateCommand {
    pub fn new(name: String, vault: Option<String>) -> CreateCommand {
        CreateCommand { name, vault }
    }

    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(Self::run_impl, (options, self))
    }

    async fn run_impl(
        _ctx: Context,
        (options, cmd): (CommandGlobalOpts, CreateCommand),
    ) -> crate::Result<()> {
        cmd.create_identity(options).await.map(|_| ())
    }

    pub async fn create_identity(
        &self,
        options: CommandGlobalOpts,
    ) -> crate::Result<IdentityIdentifier> {
        let default_vault_created = self.vault.is_none() && options.state.vaults.default().is_err();
        let vault_state = options
            .state
            .create_vault_state(self.vault.as_deref())
            .await?;
        let mut output = String::new();
        if default_vault_created {
            output.push_str(&format!("Default vault created: {}\n", &vault_state.name()));
        }

        let vault = vault_state.get().await?;

        let identity = options
            .state
            .get_identities(vault)
            .await?
            .identities_creation()
            .create_identity()
            .await?;

        options
            .state
            .create_identity_state(&identity.identifier(), Some(&self.name))
            .await?;

        let identifier = identity.identifier();
        output.push_str(&format!("Identity created: {}", identifier.clone()));

        options
            .terminal
            .stdout()
            .plain(output)
            .machine(identifier.clone())
            .json(serde_json::json!({ "identity": { "identifier": &identifier } }))
            .write_line()?;
        Ok(identifier)
    }
}
