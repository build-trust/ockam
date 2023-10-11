use std::path::PathBuf;

use crate::{
    fmt_err, fmt_log, fmt_ok, util::node_rpc, vault::default_vault_name, CommandGlobalOpts,
};
use miette::miette;

use crate::credential::identities;
use clap::Args;
use colorful::Colorful;
use ockam::identity::Identifier;
use ockam::Context;
use tokio::{sync::Mutex, try_join};

use crate::util::parsers::identity_identifier_parser;

use super::validate_encoded_cred;

#[derive(Clone, Debug, Args)]
pub struct VerifyCommand {
    #[arg(long = "issuer", value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
    pub issuer: Identifier,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,

    /// Name of the Vault that was used to issue the credential
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,
}

impl VerifyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }

    pub fn issuer(&self) -> &Identifier {
        &self.issuer
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, VerifyCommand),
) -> miette::Result<()> {
    opts.terminal
        .write_line(&fmt_log!("Verifying credential...\n"))?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let cred_as_str = match (&cmd.credential, &cmd.credential_path) {
            (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path)
                .await?
                .trim()
                .to_string(),
            (Some(credential), _) => credential.clone(),
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

        let issuer = cmd.issuer();

        let identities = match identities(&vault_name, &opts).await {
            Ok(i) => i,
            Err(_) => {
                *is_finished.lock().await = true;
                return Err(miette!("Invalid state").into());
            }
        };

        let cred = hex::decode(&cred_as_str)?;
        let is_valid = match validate_encoded_cred(&cred, identities, issuer).await {
            Ok(_) => (true, String::new()),
            Err(e) => (false, e.to_string()),
        };

        *is_finished.lock().await = true;
        Ok(is_valid)
    };

    let output_messages = vec![format!("Verifying credential...")];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let ((is_valid, reason), _) = try_join!(send_req, progress_output)?;
    let plain_text = match is_valid {
        true => fmt_ok!("Credential is valid"),
        false => fmt_err!("Credential is not valid\n") + &fmt_log!("{reason}"),
    };

    opts.terminal
        .stdout()
        .machine(is_valid.to_string())
        .json(serde_json::json!({ "is_valid": is_valid }))
        .plain(plain_text)
        .write_line()?;

    Ok(())
}
