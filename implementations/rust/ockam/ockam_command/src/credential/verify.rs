use std::path::PathBuf;

use clap::Args;
use colorful::Colorful;
use miette::miette;
use tokio::{sync::Mutex, try_join};

use ockam::Context;
use ockam_identity::{identities, Identity};

use crate::{
    CommandGlobalOpts, docs, fmt_err, fmt_log, fmt_ok, Result, util::node_rpc,
    vault::default_vault_name,
};

use super::validate_encoded_cred;

const LONG_ABOUT: &str = include_str!("./static/verify/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/verify/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
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
    (opts, cmd): (CommandGlobalOpts, VerifyCommand),
) -> crate::Result<()> {
    opts.terminal
        .write_line(&fmt_log!("Verifying credential...\n"))?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let cred_as_str = match (&cmd.credential, &cmd.credential_path) {
            (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path).await?,
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

        let issuer = match &cmd.issuer().await {
            Ok(i) => i,
            Err(_) => {
                *is_finished.lock().await = true;
                return Ok((false, "Issuer is invalid".to_string()));
            }
        }
        .identifier();

        let is_valid = match validate_encoded_cred(&cred_as_str, &issuer, &vault_name, &opts).await
        {
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
