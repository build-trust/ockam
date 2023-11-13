use std::path::PathBuf;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::identity::Identity;
use ockam::Context;
use ockam_api::cli_state::random_name;

use crate::credential::verify::verify_credential;
use crate::{fmt_log, fmt_ok, terminal::OckamColor, util::node_rpc, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct StoreCommand {
    #[arg(hide_default_value = true, default_value_t = random_name())]
    pub credential_name: String,

    /// The full hex-encoded Identity that was used to issue the credential
    #[arg(long = "issuer", value_name = "HEX_ENCODED_FULL_IDENTITY")]
    pub issuer: String,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_STRING", long)]
    pub credential: Option<String>,

    #[arg(group = "credential_value", value_name = "CREDENTIAL_FILE", long)]
    pub credential_path: Option<PathBuf>,

    /// Name of the Vault that was used to issue the credential
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,
}

impl StoreCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
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
        let issuer = verify_issuer(&opts, &cmd.issuer, &cmd.vault).await?;
        let credential_and_purpose_key = verify_credential(
            &opts,
            issuer.identifier(),
            &cmd.credential,
            &cmd.credential_path,
            &cmd.vault,
        )
        .await?;
        // store
        opts.state
            .store_credential(
                &cmd.credential_name,
                &issuer,
                credential_and_purpose_key.clone(),
            )
            .await
            .into_diagnostic()?;

        *is_finished.lock().await = true;
        Ok(credential_and_purpose_key
            .encode_as_string()
            .into_diagnostic()?)
    };

    let output_messages = vec![format!("Storing credential...")];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (credential, _) = try_join!(send_req, progress_output)?;

    opts.terminal
        .stdout()
        .machine(credential.clone())
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

async fn verify_issuer(
    opts: &CommandGlobalOpts,
    issuer: &str,
    vault: &Option<String>,
) -> miette::Result<Identity> {
    let identities = opts
        .state
        .get_identities_with_optional_vault_name(vault)
        .await?;
    let identifier = identities
        .identities_creation()
        .import(None, &hex::decode(issuer).into_diagnostic()?)
        .await
        .into_diagnostic()?;
    let identity = identities
        .get_identity(&identifier)
        .await
        .into_diagnostic()?;
    Ok(identity)
}
