use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use ockam::identity::Identity;
use ockam::Context;
use ockam_api::cli_state::traits::{StateItemDirTrait, StateTrait};
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
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
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

    pub async fn create_identity(&self, options: CommandGlobalOpts) -> crate::Result<Identity> {
        let default_vault_created = self.vault.is_none() && options.state.vaults.default().is_err();
        let vault_state = options.state.create_vault_state(self.vault.clone()).await?;
        let mut output = String::new();
        if default_vault_created {
            output.push_str(&format!("Default vault created: {}\n", &vault_state.name()));
        }

        let identity_state = options
            .state
            .create_identity_state(Some(self.name.clone()), vault_state.get().await?)
            .await?;
        let identity = identity_state.config.identity();

        output.push_str(&format!("Identity created: {}", identity.identifier()));

        options
            .terminal
            .stdout()
            .plain(output)
            .machine(identity.identifier())
            .json(serde_json::json!({ "identity": { "identifier": &identity.identifier() } }))
            .write_line()?;
        Ok(identity)
    }
}
