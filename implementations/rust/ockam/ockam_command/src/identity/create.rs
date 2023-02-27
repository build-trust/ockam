use crate::util::node_rpc;
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::cli_state::{self, VaultConfig};
use ockam_identity::Identity;
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let vault_state = if let Some(vault_name) = cmd.vault {
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
        &options.state.identities.authenticated_storage().await?,
        &vault,
    )
    .await?;
    let identity_config = cli_state::IdentityConfig::new(&identity).await;
    options
        .state
        .identities
        .create(&cmd.name, identity_config)?;
    println!("Identity created: {}", identity.identifier());
    Ok(())
}
