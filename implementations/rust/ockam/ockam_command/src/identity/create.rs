use crate::help;
use crate::util::node_rpc;
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::{
    vault::{Secret, SecretPersistence, SecretType},
    Context,
};
use ockam_api::cli_state::{self, VaultConfig};
use ockam_identity::{Identity, IdentityStateConst, KeyAttributes};
use ockam_vault::{SecretAttributes, SecretVault};
use rand::prelude::random;

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct CreateCommand {
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    name: String,

    /// Vault name to store the identity key
    #[arg(long)]
    vault: Option<String>,

    /// Use an existing AWS KMS key.
    #[arg(long)]
    key_id: Option<String>,
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
            .create(
                &vault_name,
                VaultConfig::fs_default(&vault_name, cmd.key_id.is_some())?,
            )
            .await?;
        println!("Default vault created: {}", &vault_name);
        state
    } else {
        options.state.vaults.default()?
    };
    let vault = vault_state.config.get().await?;
    let identity = if let Some(kid) = cmd.key_id {
        let attrs = SecretAttributes::new(SecretType::NistP256, SecretPersistence::Persistent, 32);
        let kid = vault
            .secret_import(Secret::Aws(kid.to_string()), attrs)
            .await?;
        let attrs = KeyAttributes::new(IdentityStateConst::ROOT_LABEL.to_string(), attrs);
        Identity::create_ext(&ctx, &vault, &kid, attrs).await?
    } else {
        Identity::create(&ctx, &vault).await?
    };
    let identity_config = cli_state::IdentityConfig::new(&identity, &vault_state.name()?).await?;
    options
        .state
        .identities
        .create(&cmd.name, identity_config)?;
    println!("Identity created: {}", identity.identifier());
    Ok(())
}
