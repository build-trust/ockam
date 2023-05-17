use std::path::PathBuf;

use crate::{util::node_rpc, vault::default_vault_name, CommandGlobalOpts, Result};
use anyhow::anyhow;

use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_identity::{identities, Identity};

use super::validate_encoded_cred;

#[derive(Clone, Debug, Args)]
pub struct VerifyCommand {
    #[arg(long = "issuer")]
    pub issuer: String,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,

    #[arg()]
    pub vault: Option<String>,
}

impl VerifyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }

    pub async fn issuer(&self) -> Result<Identity> {
        let identity_as_bytes = match hex::decode(&self.issuer) {
            Ok(b) => b,
            Err(e) => return Err(anyhow!(e).into()),
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
    (opts, cmd): (CommandGlobalOpts, VerifyCommand),
) -> crate::Result<()> {
    let cred_as_str = match (&cmd.credential, &cmd.credential_path) {
        (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path).await?,
        (Some(credential), _) => credential.clone(),
        _ => return Err(anyhow!("Credential or Credential Path argument must be provided").into()),
    };

    let vault_name = cmd
        .vault
        .clone()
        .unwrap_or_else(|| default_vault_name(&opts.state));
    match validate_encoded_cred(
        &cred_as_str,
        &cmd.issuer().await?.identifier(),
        &vault_name,
        &opts,
    )
    .await
    {
        Ok(_) => {
            println!("{} Verified Credential", "✔︎".light_green());
        }
        Err(e) => {
            println!("{} Credential is not valid!\n\n{e}", "✕".light_red());
        }
    };

    Ok(())
}
