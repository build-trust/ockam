use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use tokio::{sync::Mutex, try_join};

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::{
    ChangeHistoryRepository, ChangeHistorySqlxDatabase, CredentialsVerification, Identifier,
    PurposeKeyVerification,
};
use ockam::Context;
use ockam_api::CliState;
use ockam_vault::{SoftwareVaultForVerifyingSignatures, VaultForVerifyingSignatures};

use crate::util::parsers::identity_identifier_parser;
use crate::{
    fmt_err, fmt_log, fmt_ok, util::node_rpc, CommandGlobalOpts, Terminal, TerminalStream,
};

#[derive(Clone, Debug, Args)]
pub struct VerifyCommand {
    #[arg(long = "issuer", value_name = "IDENTIFIER", value_parser = identity_identifier_parser)]
    pub issuer: Identifier,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,
}

impl VerifyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), run_impl, (opts, self));
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
        &opts.state,
        &opts.terminal,
        cmd.issuer(),
        &cmd.credential,
        &cmd.credential_path,
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
    state: &CliState,
    terminal: &Terminal<TerminalStream<Term>>,
    issuer: &Identifier,
    credential: &Option<String>,
    credential_path: &Option<PathBuf>,
) -> miette::Result<CredentialAndPurposeKey> {
    terminal.write_line(&fmt_log!("Verifying credential...\n"))?;

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

        let change_history_repository = ChangeHistorySqlxDatabase::new(state.database());

        let result = validate_encoded_credential(
            Arc::new(change_history_repository),
            SoftwareVaultForVerifyingSignatures::create(),
            issuer,
            &credential_as_str,
        )
        .await;

        *is_finished.lock().await = true;
        Ok(result.map_err(|e| e.wrap_err("Credential is invalid"))?)
    };

    let output_messages = vec!["Verifying credential...".to_string()];

    let progress_output = terminal.progress_output(&output_messages, &is_finished);

    let (credential_and_purpose_key, _) = try_join!(send_req, progress_output)?;

    Ok(credential_and_purpose_key)
}

async fn validate_encoded_credential(
    change_history_repository: Arc<dyn ChangeHistoryRepository>,
    verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
    issuer: &Identifier,
    credential_as_str: &str,
) -> miette::Result<CredentialAndPurposeKey> {
    let credential_and_purpose_key: CredentialAndPurposeKey =
        minicbor::decode(&hex::decode(credential_as_str).into_diagnostic()?).into_diagnostic()?;
    CredentialsVerification::verify_credential_static(
        Arc::new(PurposeKeyVerification::new(
            verifying_vault.clone(),
            change_history_repository,
        )),
        verifying_vault,
        None,
        &[issuer.clone()],
        &credential_and_purpose_key,
    )
    .await
    .into_diagnostic()?;
    Ok(credential_and_purpose_key)
}
