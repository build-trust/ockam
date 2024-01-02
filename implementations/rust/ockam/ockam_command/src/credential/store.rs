use std::path::PathBuf;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::identity::Identity;
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::random_name;

use crate::credential::verify::verify_credential;
use crate::node::util::initialize_default_node;
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

    /// Store the identity attributes of the credential for the specified node
    #[arg(id = "for", value_name = "NODE_NAME", long, value_parser = extract_address_value)]
    pub node: Option<String>,
}

impl StoreCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StoreCommand),
) -> miette::Result<()> {
    // Set node name in state to store identity attributes to it
    initialize_default_node(&ctx, &opts).await?;
    let node_name = opts.state.get_node_or_default(&cmd.node).await?.name();
    let mut state = opts.state.clone();
    state.set_node_name(node_name);

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
        state
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
    let vault = opts
        .state
        .get_named_vault_or_default(vault)
        .await?
        .vault()
        .await?;
    let identities = opts.state.make_identities(vault).await?;

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
