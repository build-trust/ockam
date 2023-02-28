use std::path::PathBuf;

use crate::{util::node_rpc, vault::default_vault_name, CommandGlobalOpts};
use anyhow::anyhow;

use clap::Args;
use colorful::Colorful;
use ockam::Context;

use ockam_identity::IdentityIdentifier;

use super::validate_encoded_cred;

#[derive(Clone, Debug, Args)]
pub struct VerifyCommand {
    #[arg(long = "issuer")]
    pub issuer: IdentityIdentifier,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,

    #[arg(default_value_t = default_vault_name())]
    pub vault: String,
}

impl VerifyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, VerifyCommand),
) -> crate::Result<()> {
    let cred_as_str = match (cmd.credential, cmd.credential_path) {
        (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path).await?,
        (Some(credential), _) => credential,
        _ => return Err(anyhow!("Credential or Credential Path argument must be provided").into()),
    };

    match validate_encoded_cred(&cred_as_str, &cmd.issuer, &cmd.vault, &opts, &ctx).await {
        Ok(_) => {
            println!("{} Verified Credential", "✔︎".light_green());
        }
        Err(e) => {
            println!("{} Credential is not valid!\n\n{e}", "✕".light_red());
        }
    };

    Ok(())
}
