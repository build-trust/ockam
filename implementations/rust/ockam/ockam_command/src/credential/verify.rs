use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::{sync::Mutex, try_join};

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::{
    ChangeHistoryRepository, ChangeHistorySqlxDatabase, CredentialsVerification, Identifier,
    PurposeKeyVerification,
};
use ockam_api::{fmt_err, fmt_log, fmt_ok};
use ockam_vault::{SoftwareVaultForVerifyingSignatures, VaultForVerifyingSignatures};

use crate::util::parsers::identity_identifier_parser;
use crate::CommandGlobalOpts;

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
    pub fn name(&self) -> String {
        "credential verify".into()
    }

    pub fn issuer(&self) -> &Identifier {
        &self.issuer
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let (is_valid, plain) = match verify_credential(
            &opts,
            self.issuer(),
            &self.credential,
            &self.credential_path,
        )
        .await
        {
            Ok(_) => (true, fmt_ok!("Credential is valid")),
            Err(e) => (false, fmt_err!("{e}")),
        };

        opts.terminal
            .stdout()
            .plain(plain)
            .json(serde_json::json!({ "is_valid": is_valid }))
            .machine(is_valid.to_string())
            .write_line()?;

        Ok(())
    }
}

pub async fn verify_credential(
    opts: &CommandGlobalOpts,
    issuer: &Identifier,
    credential: &Option<String>,
    credential_path: &Option<PathBuf>,
) -> miette::Result<CredentialAndPurposeKey> {
    opts.terminal
        .write_line(fmt_log!("Verifying credential...\n"))?;

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let credential_as_str = match (&credential, &credential_path) {
            (_, Some(credential_path)) => tokio::fs::read_to_string(credential_path)
                .await
                .into_diagnostic()?
                .trim()
                .to_string(),
            (Some(credential), _) => credential.clone(),
            _ => {
                *is_finished.lock().await = true;
                return Err(miette!(
                    "Credential or Credential Path argument must be provided"
                ));
            }
        };

        let change_history_repository = ChangeHistorySqlxDatabase::new(opts.state.database());

        let result = validate_encoded_credential(
            Arc::new(change_history_repository),
            SoftwareVaultForVerifyingSignatures::create(),
            issuer,
            &credential_as_str,
        )
        .await;

        *is_finished.lock().await = true;
        result.map_err(|e| e.wrap_err("Credential is not valid"))
    };

    let output_messages = vec!["Verifying credential...".to_string()];

    let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

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
