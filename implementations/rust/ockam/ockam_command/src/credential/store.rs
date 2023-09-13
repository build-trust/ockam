use std::path::PathBuf;

use crate::{
    credential::validate_encoded_cred,
    fmt_log, fmt_ok,
    terminal::OckamColor,
    util::{node_rpc, random_name},
    vault::default_vault_name,
    CommandGlobalOpts, Result,
};
use colorful::Colorful;
use miette::miette;

use clap::Args;
use ockam::identity::{identities, Identity};
use ockam::Context;
use ockam_api::cli_state::{CredentialConfig, StateDirTrait};
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
            .import(None, &identity_as_bytes)
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

        let issuer = match cmd.identity().await {
            Ok(i) => i,
            Err(_) => {
                *is_finished.lock().await = true;
                return Err(miette!("Issuer is invalid").into());
            }
        };

        let cred = hex::decode(&cred_as_str)?;
        if let Err(e) = validate_encoded_cred(&cred, issuer.identifier(), &vault_name, &opts).await
        {
            *is_finished.lock().await = true;
            return Err(miette!("Credential is invalid\n{}", e).into());
        }

        // store
        opts.state.credentials.create(
            &cmd.credential_name,
            CredentialConfig::new(issuer.identifier().clone(), issuer.export()?, cred)?,
        )?;

        *is_finished.lock().await = true;

        Ok(cred_as_str)
    };

    let output_messages = vec![format!("Storing credential...")];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (credential, _) = try_join!(send_req, progress_output)?;

    opts.terminal
        .stdout()
        .machine(credential.to_string())
        .json(serde_json::json!(
            {
                "name": cmd.credential_name,
                "issuer": cmd.issuer,
                "credential": credential
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
