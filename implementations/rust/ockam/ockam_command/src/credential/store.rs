use std::path::PathBuf;

use crate::{
    fmt_log, fmt_ok,
    terminal::OckamColor,
    util::{node_rpc, random_name},
    CommandGlobalOpts, Result,
    credential::validate_encoded_cred,
    vault::default_vault_name,
};
use colorful::Colorful;
use miette::miette;

use clap::Args;
use ockam::Context;
use ockam_api::cli_state::{CredentialConfig, StateDirTrait};
use ockam_identity::{identities, Identity};
use tokio::{sync::Mutex, try_join};

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

    #[arg()]
    pub vault: Option<String>,
}

impl StoreCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }

    pub async fn identity(&self) -> Result<Identity> {
        let identity_as_bytes = match hex::decode(&self.issuer) {
            Ok(b) => b,
            Err(e) => return Err(miette!(e).into()),
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
    (opts, cmd): (CommandGlobalOpts, StoreCommand),
) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!(
        "Storing credential {}...\n",
        cmd.credential_name.clone()
    ))?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let cred_as_str = match (&cmd.credential, &cmd.credential_path) {
            (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path).await?,
            (Some(credential), _) => credential.to_string(),
            _ => {
                *is_finished.lock().await = true;
                return crate::Result::Err(
                    miette!("Credential or Credential Path argument must be provided").into(),
                );
            }
        };

        let vault_name = cmd
            .vault
            .clone()
            .unwrap_or_else(|| default_vault_name(&opts.state));

        let issuer = match &cmd.identity().await {
            Ok(i) => i,
            Err(_) => {
                *is_finished.lock().await = true;
                return Ok((false, "Issuer is invalid".to_string()));
            }
        }
        .identifier();

        match validate_encoded_cred(&cred_as_str, &issuer, &vault_name, &opts).await
        {
            Ok(_) => {
                // store
                opts.state.credentials.create(
                    &cmd.credential_name,
                    CredentialConfig::new(cmd.identity().await?, cred_as_str.clone())?,
                )?;

                *is_finished.lock().await = true;
                Ok((true, cred_as_str))
            },
            Err(e) => {
                *is_finished.lock().await = true;
                Ok((false, format!("Credential is invalid\n{}", e.to_string())))
            }
        }
    };

    let output_messages = vec![format!("Storing credential...")];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let ((is_valid, result), _) = try_join!(send_req, progress_output)?;

    if !is_valid {
        Err(miette!(result).into())
    } else {
        opts.terminal
            .stdout()
            .machine(result.to_string())
            .json(serde_json::json!(
                {
                    "name": cmd.credential_name,
                    "issuer": cmd.issuer,
                    "credential": result
                }
            ))
            .plain(fmt_ok!(
                "Credential {} stored\n",
                cmd.credential_name
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ))
            .write_line()?;

        Ok(())
    }
}
