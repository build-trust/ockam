use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::{sync::Mutex, try_join};

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::{Identifier, Identities};
use ockam::Context;

use crate::util::parsers::identity_identifier_parser;
use crate::{fmt_err, fmt_log, fmt_ok, util::node_rpc, CommandGlobalOpts};

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
    let (is_valid, plain_text) = match verify_credential(
        &opts,
        cmd.issuer(),
        &cmd.credential,
        &cmd.credential_path,
        &cmd.vault,
    )
    .await
    {
        Ok(_) => (true, fmt_ok!("Credential is valid")),
        Err(e) => (
            false,
            fmt_err!("Credential is not valid\n") + &fmt_log!("{}", e),
        ),
    };

    opts.terminal
        .stdout()
        .plain(plain_text)
        .json(serde_json::json!({ "is_valid": is_valid }))
        .machine(is_valid.to_string())
        .write_line()?;

    Ok(())
}

pub async fn verify_credential(
    opts: &CommandGlobalOpts,
    issuer: &Identifier,
    credential: &Option<String>,
    credential_path: &Option<PathBuf>,
    vault: &Option<String>,
) -> miette::Result<CredentialAndPurposeKey> {
    opts.terminal
        .write_line(&fmt_log!("Verifying credential...\n"))?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let credential_as_str = match (&credential, &credential_path) {
            (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path)
                .await?
                .trim()
                .to_string(),
            (Some(credential), _) => credential.clone(),
            _ => {
                *is_finished.lock().await = true;
                return Err(
                    miette!("Credential or Credential Path argument must be provided").into(),
                );
            }
        };

        let vault = opts
            .state
            .get_named_vault_or_default(vault)
            .await?
            .vault()
            .await?;
        let identities = match opts.state.make_identities(vault).await {
            Ok(i) => i,
            Err(e) => {
                *is_finished.lock().await = true;
                return Err(e)?;
            }
        };

        let result = validate_encoded_credential(identities, issuer, &credential_as_str).await;
        *is_finished.lock().await = true;
        Ok(result.map_err(|e| e.wrap_err("Credential is invalid"))?)
    };

    let output_messages = vec!["Verifying credential...".to_string()];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (credential_and_purpose_key, _) = try_join!(send_req, progress_output)?;

    Ok(credential_and_purpose_key)
}

async fn validate_encoded_credential(
    identities: Arc<Identities>,
    issuer: &Identifier,
    credential_as_str: &str,
) -> miette::Result<CredentialAndPurposeKey> {
    let verification = identities.credentials().credentials_verification();
    let credential_and_purpose_key: CredentialAndPurposeKey =
        minicbor::decode(&hex::decode(credential_as_str).into_diagnostic()?).into_diagnostic()?;
    verification
        .verify_credential(None, &[issuer.clone()], &credential_and_purpose_key)
        .await
        .into_diagnostic()?;
    Ok(credential_and_purpose_key)
}
