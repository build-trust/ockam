use crate::util::node_rpc;
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::cli_state::{self, VaultConfig};
use ockam_core::compat::sync::Arc;
use ockam_identity::{Identity, PublicIdentity};
use rand::prelude::random;

#[derive(Clone, Debug, Args)]
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
        ctx: Context,
        (options, cmd): (CommandGlobalOpts, CreateCommand),
    ) -> crate::Result<()> {
        cmd.create_identity(ctx, options).await.map(|_| ())
    }

    pub async fn create_identity(
        &self,
        ctx: Context,
        options: CommandGlobalOpts,
    ) -> crate::Result<PublicIdentity> {
        let vault_state = if let Some(vault_name) = self.vault.clone() {
            options.state.vaults.get(&vault_name)?
        } else if options.state.vaults.default().is_err() {
            let vault_name = hex::encode(random::<[u8; 4]>());
            let state = options
                .state
                .vaults
                .create(&vault_name, VaultConfig::default())
                .await?;
            println!("Default vault created: {}", &vault_name);
            state
        } else {
            options.state.vaults.default()?
        };
        let vault = vault_state.get().await?;
        let identity = Identity::create_ext(
            &ctx,
            options.state.identities.authenticated_storage().await?,
            Arc::new(vault),
        )
        .await?;
        let identity_config = cli_state::IdentityConfig::new(&identity).await;
        let identity_state = options
            .state
            .identities
            .create(&self.name, identity_config)?;
        println!("Identity created: {}", identity.identifier());
        Ok(identity_state.config.public_identity())
    }
}
