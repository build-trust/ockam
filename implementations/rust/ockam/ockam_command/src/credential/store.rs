use std::path::PathBuf;

use crate::{
    util::{node_rpc, random_name},
    vault::default_vault_name,
    CommandGlobalOpts, Result,
};
use anyhow::anyhow;

use clap::Args;
use ockam::Context;
use ockam_api::cli_state::CredentialConfig;
use ockam_identity::{identities, Identity};

#[derive(Clone, Debug, Args)]
pub struct StoreCommand {
    #[arg(hide_default_value = true, default_value_t = random_name())]
    pub credential_name: String,

    #[arg(long = "issuer")]
    pub issuer: String,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,

    #[arg(default_value_t = default_vault_name())]
    pub vault: String,
}

impl StoreCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }

    pub async fn identity(&self) -> Result<Identity> {
        let identity_as_bytes = match hex::decode(&self.issuer) {
            Ok(b) => b,
            Err(e) => return Err(anyhow!(e).into()),
        };
        let identity = identities()
            .identities_creation()
            .import_identity(&identity_as_bytes)
            .await?;
        Ok(identity)
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StoreCommand),
) -> crate::Result<()> {
    let cred_as_str = match (&cmd.credential, &cmd.credential_path) {
        (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path).await?,
        (Some(credential), _) => credential.to_string(),
        _ => return Err(anyhow!("Credential or Credential Path argument must be provided").into()),
    };

    // store
    opts.state
        .credentials
        .create(
            &cmd.credential_name,
            CredentialConfig::new(cmd.identity().await?, cred_as_str)?,
        )
        .await?;

    println!("Credential {} stored", &cmd.credential_name);

    Ok(())
}
